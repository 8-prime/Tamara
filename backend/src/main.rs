mod pdf;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{
    FromRow, SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    secure_cookie: bool,
    setup_hash: Option<String>,
    attempts: Arc<Mutex<HashMap<String, (u32, u64)>>>,
}
#[derive(Debug)]
struct AppError(StatusCode, String);
type Result<T> = std::result::Result<T, AppError>;
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref d) = e
            && d.is_unique_violation()
        {
            return Self(
                StatusCode::CONFLICT,
                "Dieser Eintrag existiert bereits.".into(),
            );
        }
        tracing::error!(error=%e, "database operation failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Die Daten konnten nicht gespeichert werden.".into(),
        )
    }
}
fn bad(s: &str) -> AppError {
    AppError(StatusCode::BAD_REQUEST, s.into())
}
fn missing() -> AppError {
    AppError(
        StatusCode::NOT_FOUND,
        "Nicht gefunden oder kein Zugriff.".into(),
    )
}
fn internal(e: impl std::fmt::Display) -> AppError {
    tracing::error!(error=%e);
    AppError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Ein interner Fehler ist aufgetreten.".into(),
    )
}
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
fn text(s: &str, label: &str, max: usize) -> Result<String> {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > max || s.chars().any(char::is_control) {
        return Err(bad(&format!(
            "{label}: Bitte 1 bis {max} Zeichen eingeben."
        )));
    }
    Ok(s.into())
}
fn email(s: &str) -> Result<String> {
    let s = text(s, "E-Mail", 254)?.to_lowercase();
    if !s.contains('@') || s.starts_with('@') || s.ends_with('@') || s.contains(char::is_whitespace)
    {
        return Err(bad("Bitte eine gültige E-Mail-Adresse eingeben."));
    }
    Ok(s)
}
#[derive(Serialize, FromRow)]
struct Teacher {
    id: String,
    name: String,
    email: String,
    is_admin: bool,
}
async fn auth(s: &AppState, h: &HeaderMap) -> Result<Teacher> {
    let token =
        cookie(h).ok_or_else(|| AppError(StatusCode::UNAUTHORIZED, "Bitte anmelden.".into()))?;
    sqlx::query_as::<_,Teacher>("SELECT t.id,t.name,t.email,t.is_admin FROM teachers t JOIN sessions s ON s.teacher_id=t.id WHERE s.token_hash=? AND s.expires_at>unixepoch()")
        .bind(hash(&token)).fetch_optional(&s.db).await?.ok_or_else(||AppError(StatusCode::UNAUTHORIZED,"Bitte erneut anmelden.".into()))
}
fn cookie(h: &HeaderMap) -> Option<String> {
    h.get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix("tamara_session=")
                .map(str::to_owned)
        })
}
async fn member(s: &AppState, class: &str, teacher: &str) -> Result<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT role FROM classroom_teachers WHERE classroom_id=? AND teacher_id=?",
    )
    .bind(class)
    .bind(teacher)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)
}
async fn owner(s: &AppState, class: &str, teacher: &str) -> Result<()> {
    if member(s, class, teacher).await? != "owner" {
        return Err(AppError(
            StatusCode::FORBIDDEN,
            "Nur die Klassenleitung kann Freigaben verwalten.".into(),
        ));
    }
    Ok(())
}
async fn check_period(s: &AppState, class: &str, period: &str) -> Result<()> {
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM periods WHERE id=? AND classroom_id=?")
        .bind(period)
        .bind(class)
        .fetch_one(&s.db)
        .await?
        != 1
    {
        return Err(missing());
    }
    Ok(())
}
// A custom header is required for browser mutations. No CORS is enabled, so
// foreign origins cannot send it without a rejected preflight.
async fn api_guard(req: axum::extract::Request, next: Next) -> Response {
    if !matches!(
        *req.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) && req
        .headers()
        .get("x-tamara-request")
        .and_then(|v| v.to_str().ok())
        != Some("1")
    {
        return AppError(StatusCode::FORBIDDEN, "Ungültige Anfrage.".into()).into_response();
    }
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}
#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
    name: Option<String>,
    registration_token: Option<String>,
}
fn throttle(s: &AppState, key: &str) -> Result<()> {
    let mut a = s.attempts.lock().map_err(internal)?;
    a.retain(|_, (_, t)| now().saturating_sub(*t) < 600);
    if a.len() > 10000 {
        return Err(AppError(
            StatusCode::TOO_MANY_REQUESTS,
            "Bitte später erneut versuchen.".into(),
        ));
    }
    let entry = a.entry(key.into()).or_insert((0, now()));
    if entry.0 >= 10 {
        return Err(AppError(
            StatusCode::TOO_MANY_REQUESTS,
            "Zu viele Versuche. Bitte in zehn Minuten erneut versuchen.".into(),
        ));
    }
    entry.0 += 1;
    Ok(())
}
async fn session(s: &AppState, t: Teacher) -> Result<Response> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    sqlx::query("DELETE FROM sessions WHERE expires_at<=unixepoch()")
        .execute(&s.db)
        .await?;
    sqlx::query(
        "INSERT INTO sessions(token_hash,teacher_id,expires_at) VALUES(?,?,unixepoch()+604800)",
    )
    .bind(hash(&token))
    .bind(&t.id)
    .execute(&s.db)
    .await?;
    let mut r = Json(t).into_response();
    let c = format!(
        "tamara_session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800{}",
        if s.secure_cookie { "; Secure" } else { "" }
    );
    r.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&c).map_err(internal)?,
    );
    Ok(r)
}
async fn register(State(s): State<AppState>, Json(c): Json<Credentials>) -> Result<Response> {
    let e = email(&c.email)?;
    let n = text(c.name.as_deref().unwrap_or(""), "Name", 120)?;
    if c.password.chars().count() < 12 || c.password.len() > 256 {
        return Err(bad(
            "Das Passwort muss mindestens 12 Zeichen lang sein (höchstens 256 Bytes).",
        ));
    }
    throttle(&s, &format!("register:{e}"))?;
    let password = c.password;
    let ph = tokio::task::spawn_blocking(move || {
        let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(internal)?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(internal)
    })
    .await
    .map_err(internal)??;
    let token = c.registration_token.as_deref().unwrap_or("").trim();
    if token.is_empty() || token.len() > 256 {
        return Err(bad("Bitte einen gültigen Registrierungscode eingeben."));
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teachers")
        .fetch_one(&mut *tx)
        .await?;
    let is_admin = count == 0;
    let invitation: Option<String> = if is_admin {
        if s.setup_hash.as_deref() != Some(hash(token).as_str()) {
            return Err(bad("Der Einrichtungscode ist nicht gültig."));
        }
        None
    } else {
        Some(sqlx::query_scalar::<_,String>("SELECT id FROM registration_tokens WHERE token_hash=? AND used_by IS NULL AND revoked=0 AND expires_at>unixepoch()")
            .bind(hash(token)).fetch_optional(&mut *tx).await?.ok_or_else(||bad("Der Registrierungscode ist ungültig, abgelaufen oder bereits verwendet."))?)
    };
    let t = Teacher {
        id: id(),
        name: n,
        email: e,
        is_admin,
    };
    sqlx::query("INSERT INTO teachers(id,name,email,password_hash,is_admin) VALUES(?,?,?,?,?)")
        .bind(&t.id)
        .bind(&t.name)
        .bind(&t.email)
        .bind(ph)
        .bind(is_admin)
        .execute(&mut *tx)
        .await?;
    if let Some(invitation) = invitation {
        sqlx::query("UPDATE registration_tokens SET used_by=?,used_at=unixepoch() WHERE id=?")
            .bind(&t.id)
            .bind(invitation)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    session(&s, t).await
}
async fn login(State(s): State<AppState>, Json(c): Json<Credentials>) -> Result<Response> {
    let e = email(&c.email)?;
    if c.password.len() > 256 {
        return Err(bad("Ungültige Zugangsdaten."));
    }
    throttle(&s, &format!("login:{e}"))?;
    let row = sqlx::query_as::<_, (String, String, String, String, bool)>(
        "SELECT id,name,email,password_hash,is_admin FROM teachers WHERE email=?",
    )
    .bind(&e)
    .fetch_optional(&s.db)
    .await?;
    // Verify a dummy hash as well, so unknown emails follow the expensive path.
    let ph=row.as_ref().map(|r|r.3.clone()).unwrap_or_else(||"$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$EUPjH8QwGhB2Dlmtj1OTpk7+re9ffj+pRYmPo20RUD0".into());
    let ok = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&ph).is_ok_and(|h| {
            Argon2::default()
                .verify_password(c.password.as_bytes(), &h)
                .is_ok()
        })
    })
    .await
    .map_err(internal)?;
    if !ok || row.is_none() {
        return Err(AppError(
            StatusCode::UNAUTHORIZED,
            "E-Mail oder Passwort ist nicht richtig.".into(),
        ));
    }
    s.attempts
        .lock()
        .map_err(internal)?
        .remove(&format!("login:{e}"));
    let (id, name, email, _, is_admin) = row.unwrap();
    session(
        &s,
        Teacher {
            id,
            name,
            email,
            is_admin,
        },
    )
    .await
}
async fn auth_status(State(s): State<AppState>) -> Result<Json<Value>> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teachers")
        .fetch_one(&s.db)
        .await?;
    Ok(Json(json!({"setup_required":count==0})))
}
async fn admin(s: &AppState, h: &HeaderMap) -> Result<Teacher> {
    let t = auth(s, h).await?;
    if !t.is_admin {
        return Err(AppError(
            StatusCode::FORBIDDEN,
            "Nur Administratoren können Registrierungscodes verwalten.".into(),
        ));
    }
    Ok(t)
}
#[derive(Serialize, FromRow)]
struct Invitation {
    id: String,
    label: String,
    created_at: i64,
    expires_at: i64,
    used_at: Option<i64>,
    revoked: bool,
}
async fn invitations(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Vec<Invitation>>> {
    admin(&s, &h).await?;
    Ok(Json(sqlx::query_as("SELECT id,label,created_at,expires_at,used_at,revoked FROM registration_tokens ORDER BY created_at DESC,rowid DESC").fetch_all(&s.db).await?))
}
#[derive(Deserialize)]
struct NewInvitation {
    label: String,
}
async fn create_invitation(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(c): Json<NewInvitation>,
) -> Result<Json<Value>> {
    let t = admin(&s, &h).await?;
    let label = text(&c.label, "Bezeichnung", 120)?;
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let id = id();
    sqlx::query("INSERT INTO registration_tokens(id,label,token_hash,created_by,expires_at) VALUES(?,?,?,?,unixepoch()+604800)").bind(&id).bind(label).bind(hash(&token)).bind(t.id).execute(&s.db).await?;
    Ok(Json(json!({"id":id,"token":token,"expires_in_days":7})))
}
async fn revoke_invitation(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    admin(&s, &h).await?;
    if sqlx::query("UPDATE registration_tokens SET revoked=1 WHERE id=? AND used_by IS NULL")
        .bind(id)
        .execute(&s.db)
        .await?
        .rows_affected()
        != 1
    {
        return Err(missing());
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn me(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Teacher>> {
    Ok(Json(auth(&s, &h).await?))
}
async fn logout(State(s): State<AppState>, h: HeaderMap) -> Result<Response> {
    if let Some(t) = cookie(&h) {
        sqlx::query("DELETE FROM sessions WHERE token_hash=?")
            .bind(hash(&t))
            .execute(&s.db)
            .await?;
    }
    let mut r = StatusCode::NO_CONTENT.into_response();
    r.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("tamara_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
    );
    Ok(r)
}
#[derive(Serialize, FromRow)]
struct Classroom {
    id: String,
    name: String,
    school_year: String,
    role: String,
    student_count: i64,
}
async fn classes(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Vec<Classroom>>> {
    let t = auth(&s, &h).await?;
    Ok(Json(sqlx::query_as("SELECT c.id,c.name,c.school_year,m.role,(SELECT COUNT(*) FROM students st WHERE st.classroom_id=c.id) student_count FROM classrooms c JOIN classroom_teachers m ON m.classroom_id=c.id WHERE m.teacher_id=? ORDER BY c.created_at,c.name").bind(t.id).fetch_all(&s.db).await?))
}
#[derive(Deserialize)]
struct NewClass {
    name: String,
    school_year: String,
}
async fn create_class(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(c): Json<NewClass>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    let name = text(&c.name, "Klassenname", 80)?;
    let year = text(&c.school_year, "Schuljahr", 40)?;
    let id = id();
    let mut tx = s.db.begin().await?;
    sqlx::query("INSERT INTO classrooms(id,name,school_year) VALUES(?,?,?)")
        .bind(&id)
        .bind(name)
        .bind(year)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO classroom_teachers VALUES(?,?,'owner')")
        .bind(&id)
        .bind(t.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Serialize, FromRow)]
struct Student {
    id: String,
    name: String,
}
#[derive(Serialize, FromRow)]
struct Period {
    id: String,
    name: String,
    source_period_id: Option<String>,
    created_at: i64,
}
#[derive(Serialize, FromRow)]
struct Assessment {
    student_id: String,
    criterion_id: String,
    value: String,
    updated_by: String,
    updated_at: i64,
}
#[derive(Serialize, FromRow)]
struct Collaborator {
    id: String,
    name: String,
    email: String,
    role: String,
}
#[derive(Deserialize)]
struct WorkspaceQuery {
    period_id: Option<String>,
}
async fn workspace(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Query(q): Query<WorkspaceQuery>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    let role = member(&s, &class, &t.id).await?;
    let students = sqlx::query_as::<_, Student>(
        "SELECT id,name FROM students WHERE classroom_id=? ORDER BY name COLLATE NOCASE,id",
    )
    .bind(&class)
    .fetch_all(&s.db)
    .await?;
    let periods=sqlx::query_as::<_,Period>("SELECT id,name,source_period_id,created_at FROM periods WHERE classroom_id=? ORDER BY created_at DESC,rowid DESC").bind(&class).fetch_all(&s.db).await?;
    let period = q
        .period_id
        .or_else(|| periods.first().map(|p| p.id.clone()));
    let assessments = if let Some(ref p) = period {
        check_period(&s, &class, p).await?;
        sqlx::query_as::<_,Assessment>("SELECT student_id,criterion_id,value,updated_by,updated_at FROM math_assessments WHERE period_id=?").bind(p).fetch_all(&s.db).await?
    } else {
        vec![]
    };
    let teachers=sqlx::query_as::<_,Collaborator>("SELECT t.id,t.name,t.email,m.role FROM classroom_teachers m JOIN teachers t ON m.teacher_id=t.id WHERE m.classroom_id=? ORDER BY m.role DESC,t.name").bind(&class).fetch_all(&s.db).await?;
    Ok(Json(
        json!({"students":students,"periods":periods,"period_id":period,"assessments":assessments,"teachers":teachers,"role":role}),
    ))
}
#[derive(Deserialize)]
struct NewStudents {
    names: Vec<String>,
}
async fn add_students(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(c): Json<NewStudents>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    if c.names.is_empty() || c.names.len() > 200 {
        return Err(bad("Bitte 1 bis 200 Namen eingeben."));
    }
    let names = c
        .names
        .iter()
        .map(|n| text(n, "Name", 120))
        .collect::<Result<Vec<_>>>()?;
    let mut tx = s.db.begin().await?;
    for n in names {
        sqlx::query("INSERT INTO students(id,classroom_id,name) VALUES(?,?,?)")
            .bind(id())
            .bind(&class)
            .bind(n)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"added":c.names.len()})))
}
#[derive(Deserialize)]
struct Name {
    name: String,
}
async fn rename_student(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, student)): Path<(String, String)>,
    Json(n): Json<Name>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    if sqlx::query("UPDATE students SET name=? WHERE id=? AND classroom_id=?")
        .bind(text(&n.name, "Name", 120)?)
        .bind(student)
        .bind(class)
        .execute(&s.db)
        .await?
        .rows_affected()
        != 1
    {
        return Err(missing());
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn remove_student(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, student)): Path<(String, String)>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    owner(&s, &class, &t.id).await?;
    if sqlx::query("DELETE FROM students WHERE id=? AND classroom_id=?")
        .bind(student)
        .bind(class)
        .execute(&s.db)
        .await?
        .rows_affected()
        != 1
    {
        return Err(missing());
    }
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
struct NewPeriod {
    name: String,
    source_period_id: Option<String>,
}
async fn create_period(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(p): Json<NewPeriod>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let name = text(&p.name, "Zeitraum", 120)?;
    let id = id();
    if let Some(ref source) = p.source_period_id {
        check_period(&s, &class, source).await?;
    }
    let mut tx = s.db.begin().await?;
    sqlx::query(
        "INSERT INTO periods(id,classroom_id,name,source_period_id,created_by) VALUES(?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&class)
    .bind(name)
    .bind(&p.source_period_id)
    .bind(&t.id)
    .execute(&mut *tx)
    .await?;
    if let Some(source) = p.source_period_id {
        sqlx::query("INSERT INTO math_assessments(classroom_id,period_id,student_id,criterion_id,value,updated_by,updated_at) SELECT classroom_id,?,student_id,criterion_id,value,?,unixepoch() FROM math_assessments WHERE period_id=? AND classroom_id=?").bind(&id).bind(&t.id).bind(source).bind(&class).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}
#[derive(Deserialize)]
struct Change {
    student_id: String,
    criterion_id: String,
    value: Option<String>,
}
#[derive(Deserialize)]
struct Changes {
    changes: Vec<Change>,
}
async fn assess(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, period)): Path<(String, String)>,
    Json(c): Json<Changes>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    check_period(&s, &class, &period).await?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad("Bitte 1 bis 5000 Bewertungen pro Anfrage senden."));
    }
    let students: HashSet<String> =
        sqlx::query_scalar("SELECT id FROM students WHERE classroom_id=?")
            .bind(&class)
            .fetch_all(&s.db)
            .await?
            .into_iter()
            .collect();
    let criteria: HashSet<String> = pdf::catalog()
        .criteria
        .iter()
        .map(|c| c.id.clone())
        .collect();
    for ch in &c.changes {
        if !students.contains(&ch.student_id) || !criteria.contains(&ch.criterion_id) {
            return Err(bad("Unbekanntes Kind oder unbekannte Kompetenz."));
        }
        if ch.value.as_deref().is_some_and(|v| v != "yes" && v != "no") {
            return Err(bad("Die Bewertung muss yes, no oder null sein."));
        }
    }
    let mut tx = s.db.begin().await?;
    for ch in c.changes {
        if let Some(value) = ch.value {
            sqlx::query("INSERT INTO math_assessments(classroom_id,period_id,student_id,criterion_id,value,updated_by) VALUES(?,?,?,?,?,?) ON CONFLICT(period_id,student_id,criterion_id) DO UPDATE SET value=excluded.value,updated_by=excluded.updated_by,updated_at=unixepoch()")
                .bind(&class).bind(&period).bind(ch.student_id).bind(ch.criterion_id).bind(value).bind(&t.id).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM math_assessments WHERE period_id=? AND student_id=? AND criterion_id=?").bind(&period).bind(ch.student_id).bind(ch.criterion_id).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
struct Share {
    email: String,
}
async fn share(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(c): Json<Share>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    owner(&s, &class, &t.id).await?;
    let teacher=sqlx::query_scalar::<_,String>("SELECT id FROM teachers WHERE email=?").bind(email(&c.email)?).fetch_optional(&s.db).await?.ok_or_else(||bad("Für diese E-Mail gibt es noch kein Konto. Die Lehrkraft muss sich zuerst registrieren."))?;
    sqlx::query("INSERT INTO classroom_teachers(classroom_id,teacher_id,role) VALUES(?,?,'editor') ON CONFLICT DO NOTHING").bind(class).bind(teacher).execute(&s.db).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn unshare(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, teacher)): Path<(String, String)>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    owner(&s, &class, &t.id).await?;
    if t.id == teacher {
        return Err(bad("Die Klassenleitung kann nicht entfernt werden."));
    }
    sqlx::query(
        "DELETE FROM classroom_teachers WHERE classroom_id=? AND teacher_id=? AND role='editor'",
    )
    .bind(class)
    .bind(teacher)
    .execute(&s.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn criteria(State(s): State<AppState>, h: HeaderMap) -> Result<Json<&'static pdf::Catalog>> {
    auth(&s, &h).await?;
    Ok(Json(pdf::catalog()))
}
async fn export_pdf(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, period, student)): Path<(String, String, String)>,
) -> Result<Response> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    check_period(&s, &class, &period).await?;
    let name =
        sqlx::query_scalar::<_, String>("SELECT name FROM students WHERE id=? AND classroom_id=?")
            .bind(&student)
            .bind(&class)
            .fetch_optional(&s.db)
            .await?
            .ok_or_else(missing)?;
    let year = sqlx::query_scalar::<_, String>("SELECT school_year FROM classrooms WHERE id=?")
        .bind(&class)
        .fetch_one(&s.db)
        .await?;
    let period_name = sqlx::query_scalar::<_, String>("SELECT name FROM periods WHERE id=?")
        .bind(&period)
        .fetch_one(&s.db)
        .await?;
    let yes=sqlx::query_scalar::<_,String>("SELECT criterion_id FROM math_assessments WHERE period_id=? AND student_id=? AND value='yes'").bind(period).bind(student).fetch_all(&s.db).await?;
    let bytes = tokio::task::spawn_blocking(move || pdf::render(&name, &year, &period_name, &yes))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"entwicklungsuebersicht-mathematik.pdf\""),
    );
    Ok(r)
}
fn routes(s: AppState) -> Router {
    let dist = std::env::var("TAMARA_FRONTEND_DIR").unwrap_or_else(|_| "frontend/dist".into());
    routes_with_frontend(s, std::path::Path::new(&dist))
}

fn routes_with_frontend(s: AppState, dist: &std::path::Path) -> Router {
    let api = Router::new()
        .route("/auth/status", get(auth_status))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route(
            "/admin/invitations",
            get(invitations).post(create_invitation),
        )
        .route("/admin/invitations/{id}", delete(revoke_invitation))
        .route("/classrooms", get(classes).post(create_class))
        .route("/criteria", get(criteria))
        .route("/classrooms/{class}/workspace", get(workspace))
        .route("/classrooms/{class}/students", post(add_students))
        .route(
            "/classrooms/{class}/students/{student}",
            patch(rename_student).delete(remove_student),
        )
        .route("/classrooms/{class}/periods", post(create_period))
        .route(
            "/classrooms/{class}/periods/{period}/assessments",
            patch(assess),
        )
        .route("/classrooms/{class}/teachers", post(share))
        .route("/classrooms/{class}/teachers/{teacher}", delete(unshare))
        .route(
            "/classrooms/{class}/periods/{period}/students/{student}/pdf",
            get(export_pdf),
        )
        .fallback(|| async {
            AppError(StatusCode::NOT_FOUND, "API-Endpunkt nicht gefunden.".into())
        })
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn(api_guard));
    Router::new()
        .nest("/api", api)
        // Missing bundles must stay 404s rather than return the SPA's HTML.
        .nest_service("/assets", ServeDir::new(dist.join("assets")))
        .fallback_service(ServeDir::new(dist).fallback(ServeFile::new(dist.join("index.html"))))
        .with_state(s)
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tamara=info,tower_http=info".into()),
        )
        .init();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/tamara.sqlite".into());
    let options: SqliteConnectOptions = url
        .parse::<SqliteConnectOptions>()?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10));
    if let Some(parent) = options
        .get_filename()
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    let mut tx = db.begin().await?;
    for c in &pdf::catalog().criteria {
        sqlx::query("INSERT INTO math_criteria(id,label,domain,section,level,page,x,y) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET label=excluded.label,domain=excluded.domain,section=excluded.section,level=excluded.level,page=excluded.page,x=excluded.x,y=excluded.y")
            .bind(&c.id).bind(&c.label).bind(&c.domain).bind(&c.section).bind(&c.level).bind(c.page).bind(c.x).bind(c.y).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teachers")
        .fetch_one(&db)
        .await?;
    let setup_hash = if count == 0 {
        let token = std::env::var("TAMARA_SETUP_TOKEN")
            .unwrap_or_else(|_| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));
        if token.len() < 24 {
            return Err("TAMARA_SETUP_TOKEN must contain at least 24 characters".into());
        }
        // Shown only on the local server terminal until the initial admin exists.
        eprintln!("\nTamara first-run setup token (creates the first administrator):\n{token}\n");
        Some(hash(&token))
    } else {
        None
    };
    let s = AppState {
        db,
        setup_hash,
        secure_cookie: std::env::var("TAMARA_SECURE_COOKIES").as_deref() == Ok("true"),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let addr = std::env::var("TAMARA_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Tamara is ready at http://{addr}");
    axum::serve(listener, routes(s))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests;

use super::*;
use axum::body::Bytes;

pub(super) const MAX_WORKSHEET: usize = 10 * 1024 * 1024;

#[derive(Serialize, FromRow)]
pub(super) struct Subject {
    id: String,
    name: String,
}

pub(super) async fn subjects(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
) -> Result<Json<Vec<Subject>>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    Ok(Json(
        sqlx::query_as(
            "SELECT id,name FROM subjects WHERE classroom_id=? ORDER BY name COLLATE NOCASE,id",
        )
        .bind(class)
        .fetch_all(&s.db)
        .await?,
    ))
}

pub(super) async fn create_subject(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(n): Json<Name>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let name = text(&n.name, "Fach", 80)?;
    let id = id();
    sqlx::query("INSERT INTO subjects(id,classroom_id,name) VALUES(?,?,?)")
        .bind(&id)
        .bind(class)
        .bind(&name)
        .execute(&s.db)
        .await?;
    Ok(Json(json!({"id":id,"name":name})))
}

pub(super) async fn rename_subject(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, subject)): Path<(String, String)>,
    Json(n): Json<Name>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    if sqlx::query("UPDATE subjects SET name=? WHERE id=? AND classroom_id=?")
        .bind(text(&n.name, "Fach", 80)?)
        .bind(subject)
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

pub(super) async fn remove_subject(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, subject)): Path<(String, String)>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let used: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM homework WHERE subject_id=? AND classroom_id=?")
            .bind(&subject)
            .bind(&class)
            .fetch_one(&mut *tx)
            .await?;
    if used > 0 {
        return Err(AppError(
            StatusCode::CONFLICT,
            "Dieses Fach wird für Hausaufgaben verwendet und kann nicht entfernt werden.".into(),
        ));
    }
    if sqlx::query("DELETE FROM subjects WHERE id=? AND classroom_id=?")
        .bind(subject)
        .bind(class)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        != 1
    {
        return Err(missing());
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, FromRow)]
pub(super) struct Homework {
    id: String,
    subject_id: String,
    subject_name: String,
    title: String,
    description: String,
    due_date: String,
    status: String,
    closed_at: Option<i64>,
    created_at: i64,
    assigned_count: i64,
    handed_in_count: i64,
    worksheet_name: Option<String>,
}

const SELECT_HOMEWORK: &str = "SELECT h.id,h.subject_id,s.name subject_name,h.title,h.description,h.due_date,h.status,h.closed_at,h.created_at,
    (SELECT COUNT(*) FROM homework_submissions a WHERE a.homework_id=h.id) assigned_count,
    (SELECT COUNT(*) FROM homework_submissions a WHERE a.homework_id=h.id AND a.handed_in_at IS NOT NULL) handed_in_count,
    (SELECT name FROM homework_worksheets w WHERE w.homework_id=h.id) worksheet_name
    FROM homework h JOIN subjects s ON s.id=h.subject_id";

pub(super) async fn list(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
) -> Result<Json<Vec<Homework>>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    Ok(Json(
        sqlx::query_as(&format!(
            "{SELECT_HOMEWORK} WHERE h.classroom_id=? ORDER BY h.due_date,h.created_at,h.id"
        ))
        .bind(class)
        .fetch_all(&s.db)
        .await?,
    ))
}

#[derive(Serialize, FromRow)]
struct Submission {
    student_id: String,
    name: String,
    handed_in_at: Option<i64>,
}

pub(super) async fn detail(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let mut tx = s.db.begin().await?;
    let entry: Homework = sqlx::query_as(&format!(
        "{SELECT_HOMEWORK} WHERE h.classroom_id=? AND h.id=?"
    ))
    .bind(&class)
    .bind(&homework)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(missing)?;
    let pupils = sqlx::query_as::<_, Submission>("SELECT a.student_id,s.name,a.handed_in_at FROM homework_submissions a JOIN students s ON s.id=a.student_id WHERE a.homework_id=? AND a.classroom_id=? ORDER BY s.name COLLATE NOCASE,s.id")
        .bind(homework).bind(class).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(json!({"homework":entry,"submissions":pupils})))
}

#[derive(Deserialize)]
pub(super) struct HomeworkInput {
    subject_id: String,
    title: String,
    #[serde(default)]
    description: String,
    due_date: String,
}

fn validate(c: &HomeworkInput) -> Result<(String, String)> {
    let title = text(&c.title, "Titel", 200)?;
    let description = c.description.trim();
    if description.chars().count() > 10000
        || description
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(bad(
            "Die Beschreibung darf höchstens 10000 Zeichen enthalten.",
        ));
    }
    if !valid_date(&c.due_date) {
        return Err(bad("Bitte ein gültiges Abgabedatum angeben."));
    }
    Ok((title, description.into()))
}

async fn check_subject(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
    subject: &str,
) -> Result<()> {
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM subjects WHERE id=? AND classroom_id=?")
        .bind(subject)
        .bind(class)
        .fetch_one(&mut **tx)
        .await?
        != 1
    {
        return Err(bad("Bitte ein Fach dieser Klasse auswählen."));
    }
    Ok(())
}

pub(super) async fn create(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(c): Json<HomeworkInput>,
) -> Result<Json<Value>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let (title, description) = validate(&c)?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_subject(&mut tx, &class, &c.subject_id).await?;
    let id = id();
    sqlx::query("INSERT INTO homework(id,classroom_id,subject_id,title,description,due_date,created_by) VALUES(?,?,?,?,?,?,?)")
        .bind(&id).bind(&class).bind(c.subject_id).bind(title).bind(description).bind(c.due_date).bind(t.id)
        .execute(&mut *tx).await?;
    let assigned = sqlx::query("INSERT INTO homework_submissions(homework_id,classroom_id,student_id) SELECT ?,classroom_id,id FROM students WHERE classroom_id=?")
        .bind(&id).bind(class).execute(&mut *tx).await?.rows_affected();
    if assigned == 0 {
        return Err(bad("Bitte zuerst Kinder zur Klasse hinzufügen."));
    }
    tx.commit().await?;
    Ok(Json(json!({"id":id})))
}

pub(super) async fn edit(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
    Json(c): Json<HomeworkInput>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let (title, description) = validate(&c)?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_subject(&mut tx, &class, &c.subject_id).await?;
    if sqlx::query("UPDATE homework SET subject_id=?,title=?,description=?,due_date=? WHERE id=? AND classroom_id=?")
        .bind(c.subject_id).bind(title).bind(description).bind(c.due_date).bind(homework).bind(class)
        .execute(&mut *tx).await?.rows_affected() != 1 {
        return Err(missing());
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// Reconcile under the same write transaction as roster/submission changes.
// Manual closures remain closed even when a submission is corrected.
pub(super) async fn reconcile(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
) -> Result<()> {
    sqlx::query("UPDATE homework SET status=CASE WHEN EXISTS(SELECT 1 FROM homework_submissions a WHERE a.homework_id=homework.id AND a.handed_in_at IS NULL) OR NOT EXISTS(SELECT 1 FROM homework_submissions a WHERE a.homework_id=homework.id) THEN 'open' ELSE 'auto_closed' END,
        closed_at=CASE WHEN EXISTS(SELECT 1 FROM homework_submissions a WHERE a.homework_id=homework.id AND a.handed_in_at IS NULL) OR NOT EXISTS(SELECT 1 FROM homework_submissions a WHERE a.homework_id=homework.id) THEN NULL ELSE COALESCE(closed_at,unixepoch()) END
        WHERE classroom_id=? AND status!='manually_closed'")
        .bind(class).execute(&mut **tx).await?;
    Ok(())
}

#[derive(Deserialize)]
struct SubmissionChange {
    student_id: String,
    handed_in: bool,
}
#[derive(Deserialize)]
pub(super) struct SubmissionChanges {
    changes: Vec<SubmissionChange>,
}

async fn check_homework(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    class: &str,
    homework: &str,
) -> Result<()> {
    if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM homework WHERE id=? AND classroom_id=?")
        .bind(homework)
        .bind(class)
        .fetch_one(&mut **tx)
        .await?
        != 1
    {
        return Err(missing());
    }
    Ok(())
}

pub(super) async fn save_submissions(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
    Json(c): Json<SubmissionChanges>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad("Bitte 1 bis 5000 Abgaben pro Anfrage senden."));
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_homework(&mut tx, &class, &homework).await?;
    let mut seen = HashSet::new();
    for ch in c.changes {
        if !seen.insert(ch.student_id.clone()) {
            return Err(bad("Ein Kind wurde mehrfach angegeben."));
        }
        if sqlx::query("UPDATE homework_submissions SET handed_in_at=CASE WHEN ? THEN COALESCE(handed_in_at,unixepoch()) ELSE NULL END,updated_by=? WHERE homework_id=? AND classroom_id=? AND student_id=?")
            .bind(ch.handed_in).bind(&t.id).bind(&homework).bind(&class).bind(ch.student_id)
            .execute(&mut *tx).await?.rows_affected() != 1 {
            return Err(bad("Dieses Kind gehört nicht zu dieser Hausaufgabe."));
        }
    }
    reconcile(&mut tx, &class).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(super) struct Closure {
    closed: bool,
}

pub(super) async fn set_closed(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
    Json(c): Json<Closure>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_homework(&mut tx, &class, &homework).await?;
    if !c.closed {
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM homework_submissions WHERE homework_id=? AND handed_in_at IS NULL")
            .bind(&homework).fetch_one(&mut *tx).await?;
        if remaining == 0 {
            return Err(bad(
                "Alle Kinder haben abgegeben. Korrigiere zuerst eine Abgabe, um die Hausaufgabe wieder zu öffnen.",
            ));
        }
    }
    sqlx::query("UPDATE homework SET status=?,closed_at=CASE WHEN ? THEN unixepoch() ELSE NULL END WHERE id=? AND classroom_id=?")
        .bind(if c.closed { "manually_closed" } else { "open" }).bind(c.closed).bind(homework).bind(class)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(super) struct WorksheetQuery {
    name: String,
}

pub(super) async fn upload_worksheet(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
    Query(q): Query<WorksheetQuery>,
    bytes: Bytes,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let name = text(&q.name, "Dateiname", 200)?;
    if !name.to_ascii_lowercase().ends_with(".pdf")
        || name.contains(['/', '\\'])
        || !bytes.starts_with(b"%PDF-")
        || !bytes[bytes.len().saturating_sub(1024)..]
            .windows(5)
            .any(|w| w == b"%%EOF")
    {
        return Err(bad("Bitte eine PDF-Datei (höchstens 10 MB) auswählen."));
    }
    if bytes.len() > MAX_WORKSHEET {
        return Err(AppError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Die PDF-Datei darf höchstens 10 MB groß sein.".into(),
        ));
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_homework(&mut tx, &class, &homework).await?;
    sqlx::query("INSERT INTO homework_worksheets(homework_id,name,content) VALUES(?,?,?) ON CONFLICT(homework_id) DO UPDATE SET name=excluded.name,content=excluded.content")
        .bind(homework).bind(name).bind(bytes.as_ref()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn worksheet(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
) -> Result<Response> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let (name, content): (String, Vec<u8>) = sqlx::query_as("SELECT w.name,w.content FROM homework_worksheets w JOIN homework h ON h.id=w.homework_id WHERE h.id=? AND h.classroom_id=?")
        .bind(homework).bind(class).fetch_optional(&s.db).await?.ok_or_else(missing)?;
    let encoded: String = name.bytes().map(|b| format!("%{b:02X}")).collect();
    let mut r = Response::new(Body::from(content));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"arbeitsblatt.pdf\"; filename*=UTF-8''{encoded}"
        ))
        .map_err(internal)?,
    );
    Ok(r)
}

pub(super) async fn remove_worksheet(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, homework)): Path<(String, String)>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    check_homework(&mut tx, &class, &homework).await?;
    sqlx::query("DELETE FROM homework_worksheets WHERE homework_id=?")
        .bind(homework)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

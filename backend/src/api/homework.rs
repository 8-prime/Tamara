use super::{auth::Authenticated, classrooms::Name};
use crate::{
    error::{Error, ErrorKind, Result, bad, internal},
    models::{Homework, HomeworkInput, Subject, SubmissionChange},
    state::AppState,
    util::id,
    validation::{text, valid_date},
};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) const MAX_WORKSHEET: usize = 10 * 1024 * 1024;
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

#[derive(Deserialize)]
pub(crate) struct SubmissionChanges {
    pub(crate) changes: Vec<SubmissionChange>,
}

#[derive(Deserialize)]
pub(crate) struct Closure {
    pub(crate) closed: bool,
}

#[derive(Deserialize)]
pub(crate) struct WorksheetQuery {
    pub(crate) name: String,
}

pub(super) async fn subjects(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
) -> Result<Json<Vec<Subject>>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    Ok(Json(s.db.subjects(&class).await?))
}

pub(super) async fn create_subject(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(n): Json<Name>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let name = text(&n.name, "Fach", 80)?;
    let id = id();
    s.db.create_subject(&id, &class, &name).await?;
    Ok(Json(json!({"id":id,"name":name})))
}

pub(super) async fn rename_subject(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, subject)): Path<(String, String)>,
    Json(n): Json<Name>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let name = text(&n.name, "Fach", 80)?;
    s.db.rename_subject(&class, &subject, &name).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn remove_subject(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, subject)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.remove_subject(&class, &subject).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn list(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
) -> Result<Json<Vec<Homework>>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    Ok(Json(s.db.homework(&class).await?))
}

pub(super) async fn detail(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let (entry, pupils) = s.db.homework_detail(&class, &homework).await?;
    Ok(Json(json!({"homework":entry,"submissions":pupils})))
}

pub(super) async fn create(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(mut c): Json<HomeworkInput>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    (c.title, c.description) = validate(&c)?;
    let id = s.db.create_homework(&class, &t.id, &c).await?;
    Ok(Json(json!({"id":id})))
}

pub(super) async fn edit(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
    Json(mut c): Json<HomeworkInput>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    (c.title, c.description) = validate(&c)?;
    s.db.edit_homework(&class, &homework, &c).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn save_submissions(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
    Json(c): Json<SubmissionChanges>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad("Bitte 1 bis 5000 Abgaben pro Anfrage senden."));
    }
    s.db.save_submissions(&class, &homework, &t.id, c.changes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn set_closed(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
    Json(c): Json<Closure>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.set_homework_closed(&class, &homework, c.closed)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn upload_worksheet(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
    Query(q): Query<WorksheetQuery>,
    bytes: Bytes,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
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
        return Err(Error(
            ErrorKind::PayloadTooLarge,
            "Die PDF-Datei darf höchstens 10 MB groß sein.".into(),
        ));
    }
    s.db.upload_worksheet(&class, &homework, &name, &bytes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn worksheet(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
) -> Result<Response> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let (name, content) = s.db.worksheet(&class, &homework).await?;
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
    Authenticated(t): Authenticated,
    Path((class, homework)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.remove_worksheet(&class, &homework).await?;
    Ok(StatusCode::NO_CONTENT)
}

use super::auth::Authenticated;
use crate::{
    error::{Result, bad, internal},
    state::AppState,
    validation,
};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
};
use serde::Deserialize;
use serde_json::Value;
pub(super) const MAX_DOCUMENT: usize = 10 * 1024 * 1024;
#[derive(Deserialize)]
pub(super) struct Input {
    student_id: String,
    date: String,
    content: String,
    attendees: Vec<String>,
}
#[derive(Deserialize)]
pub(super) struct Review {
    period_id: String,
}
#[derive(Deserialize)]
pub(super) struct Document {
    name: String,
}
pub(super) async fn pupils(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<Vec<Value>>> {
    Ok(Json(s.db.parent_pupils(&t.id).await?))
}
pub(super) async fn list(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<Vec<Value>>> {
    Ok(Json(s.db.parent_conferences(&t.id).await?))
}
pub(super) async fn save(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(mut input): Json<Input>,
) -> Result<StatusCode> {
    validation::text(&id, "ID", 100)?;
    if !validation::valid_date(&input.date) {
        return Err(bad("Bitte ein gültiges Datum eingeben."));
    }
    if input.content.chars().count() > 100000
        || input
            .content
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(bad("Die Notiz darf höchstens 100000 Zeichen enthalten."));
    }
    if input.attendees.len() > 20 {
        return Err(bad("Höchstens 20 anwesende Eltern/Erziehungsberechtigte."));
    }
    input.attendees = input
        .attendees
        .iter()
        .map(|n| validation::text(n, "Name", 200))
        .collect::<Result<Vec<_>>>()?;
    s.db.save_parent_conference(
        &t.id,
        &id,
        &input.student_id,
        &input.date,
        &input.content,
        &input.attendees,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn remove(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.db.remove_parent_conference(&t.id, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn capture(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(input): Json<Review>,
) -> Result<StatusCode> {
    s.db.capture_parent_review(&t.id, &id, &input.period_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn remove_review(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.db.remove_parent_review(&t.id, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn documents(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<Vec<Value>>> {
    Ok(Json(s.db.parent_documents(&t.id, &id).await?))
}
pub(super) async fn upload(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Query(q): Query<Document>,
    bytes: Bytes,
) -> Result<StatusCode> {
    let name = validation::text(&q.name, "Dateiname", 200)?;
    if name.contains(['/', '\\']) || bytes.is_empty() {
        return Err(bad(
            "Bitte eine nicht leere Datei mit gültigem Dateinamen auswählen.",
        ));
    }
    if bytes.len() > MAX_DOCUMENT {
        return Err(bad("Dokumente dürfen höchstens 10 MB groß sein."));
    }
    s.db.add_parent_document(&t.id, &id, &name, &bytes).await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn download(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((id, document)): Path<(String, String)>,
) -> Result<Response> {
    let (name, bytes) = s.db.parent_document(&t.id, &id, &document).await?;
    let encoded: String = name.bytes().map(|b| format!("%{b:02X}")).collect();
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"dokument\"; filename*=UTF-8''{encoded}"
        ))
        .map_err(internal)?,
    );
    Ok(r)
}
pub(super) async fn remove_document(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((id, document)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.db.remove_parent_document(&t.id, &id, &document).await?;
    Ok(StatusCode::NO_CONTENT)
}

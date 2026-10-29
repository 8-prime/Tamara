use super::auth::Authenticated;
use crate::{
    error::{Result, bad},
    models::Note,
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

fn content(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 10000
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(bad("Bitte eine Notiz mit 1 bis 10000 Zeichen eingeben."));
    }
    Ok(value.into())
}

#[derive(Deserialize)]
pub(crate) struct NewNote {
    pub(crate) student_id: String,
    pub(crate) content: String,
}

#[derive(Deserialize)]
pub(crate) struct EditNote {
    pub(crate) content: String,
}

pub(super) async fn list(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
) -> Result<Json<Vec<Note>>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let notes = s.db.notes(&class).await?;
    Ok(Json(notes))
}

pub(super) async fn create(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(input): Json<NewNote>,
) -> Result<Json<Note>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let content = content(&input.content)?;
    let note =
        s.db.create_note(&class, &input.student_id, &content, &t.id)
            .await?;
    Ok(Json(note))
}

pub(super) async fn edit(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, note)): Path<(String, String)>,
    Json(input): Json<EditNote>,
) -> Result<Json<Note>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let content = content(&input.content)?;
    let note = s.db.edit_note(&class, &note, &content, &t.id).await?;
    Ok(Json(note))
}

pub(super) async fn remove(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, note)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.remove_note(&class, &note).await?;
    Ok(StatusCode::NO_CONTENT)
}

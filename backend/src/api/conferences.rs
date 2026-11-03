use super::auth::Authenticated;
use crate::{
    error::{Result, bad},
    models::Conference,
    state::AppState,
    validation,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
pub(super) async fn list(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<Vec<Conference>>> {
    Ok(Json(s.db.conferences(&t.id).await?))
}
pub(super) async fn save(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(mut input): Json<Conference>,
) -> Result<Json<Conference>> {
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
    input.title = input
        .title
        .filter(|s| !s.trim().is_empty())
        .map(|s| validation::text(&s, "Titel", 200))
        .transpose()?;
    if let Some(class) = &input.classroom_id {
        s.auth.member(&s.db, class, &t.id).await?;
    }
    Ok(Json(s.db.save_conference(&t.id, &id, &input).await?))
}
pub(super) async fn remove(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.db.remove_conference(&t.id, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

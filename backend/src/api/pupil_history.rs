use super::auth::Authenticated;
use crate::{error::Result, state::AppState};
use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;

pub(super) async fn history(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    Ok(Json(s.db.pupil_history(&class).await?))
}

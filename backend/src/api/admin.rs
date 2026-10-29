use super::auth::Administrator;
use crate::{
    error::{Error, ErrorKind, Result, internal},
    models::Invitation,
    state::AppState,
    util::id,
    validation::text,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub(crate) struct NewInvitation {
    pub(crate) label: String,
}

pub(super) async fn create_backup(
    State(s): State<AppState>,
    _admin: Administrator,
) -> Result<StatusCode> {
    let backups = s.backups.as_ref().ok_or_else(|| {
        Error(
            ErrorKind::Unavailable,
            "Datenbanksicherungen sind nicht verfügbar.".into(),
        )
    })?;
    backups.manual().await.map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn invitations(
    State(s): State<AppState>,
    _admin: Administrator,
) -> Result<Json<Vec<Invitation>>> {
    Ok(Json(s.db.invitations().await?))
}

pub(super) async fn create_invitation(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Json(c): Json<NewInvitation>,
) -> Result<Json<Value>> {
    let label = text(&c.label, "Bezeichnung", 120)?;
    let token = crate::auth::token();
    let id = id();
    s.db.create_invitation(&id, &label, &crate::auth::hash(&token), &t.id)
        .await?;
    Ok(Json(json!({"id":id,"token":token,"expires_in_days":7})))
}

pub(super) async fn revoke_invitation(
    State(s): State<AppState>,
    _admin: Administrator,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.db.revoke_invitation(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

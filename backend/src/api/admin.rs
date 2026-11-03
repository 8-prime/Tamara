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
    Administrator(t): Administrator,
) -> Result<StatusCode> {
    let backups = s.backups.as_ref().ok_or_else(|| {
        Error(
            ErrorKind::Unavailable,
            "Datenbanksicherungen sind nicht verfügbar.".into(),
        )
    })?;
    backups.manual().await.map_err(internal)?;
    tracing::info!(target: "tamara::security", event="backup_created", actor_id=%t.id);
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
    tracing::info!(target: "tamara::security", event="invitation_created", actor_id=%t.id, invitation_id=%id);
    Ok(Json(json!({"id":id,"token":token,"expires_in_days":7})))
}

pub(super) async fn revoke_invitation(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.db.revoke_invitation(&id).await?;
    tracing::info!(target: "tamara::security", event="invitation_revoked", actor_id=%t.id, invitation_id=%id);
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn accounts(
    State(s): State<AppState>,
    _admin: Administrator,
) -> Result<Json<Vec<crate::models::AccountSummary>>> {
    Ok(Json(s.db.accounts().await?))
}
pub(super) async fn deletion_preview(
    State(s): State<AppState>,
    _admin: Administrator,
    Path(id): Path<String>,
) -> Result<Json<crate::models::AccountDeletion>> {
    Ok(Json(s.db.account_deletion(&id).await?))
}
pub(super) async fn delete_account(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Path(id): Path<String>,
    Json(input): Json<super::auth::DeleteAccount>,
) -> Result<StatusCode> {
    let proof = s.auth.confirm_password(&s.db, &t, input.password).await?;
    s.db.delete_account(&t.id, &id, &proof, &input.owned_classroom_ids)
        .await?;
    tracing::info!(target: "tamara::security", event="account_deleted", actor_id=%t.id, account_id=%id);
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub(super) struct AccountRole {
    is_admin: bool,
}
pub(super) async fn set_role(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Path(id): Path<String>,
    Json(input): Json<AccountRole>,
) -> Result<StatusCode> {
    s.db.set_account_admin(&t.id, &id, input.is_admin).await?;
    tracing::info!(target: "tamara::security", event="account_role_changed", actor_id=%t.id, account_id=%id, is_admin=input.is_admin);
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn password_reset(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let token = crate::auth::token();
    let expires =
        s.db.issue_password_reset(&t.id, &id, &crate::auth::hash(&token))
            .await?;
    tracing::info!(target: "tamara::security", event="password_reset_issued", actor_id=%t.id, account_id=%id);
    Ok(Json(json!({"token":token,"expires_at":expires})))
}

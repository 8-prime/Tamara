use super::auth::Administrator;
use crate::{
    db::usage::{Bucket, Usage},
    error::{Error, ErrorKind, Result, bad, internal},
    models::Invitation,
    state::AppState,
    util::id,
    validation::{email, text, valid_date},
};
use axum::{
    Json,
    extract::{Path, Query, State},
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
#[derive(Deserialize)]
pub(super) struct AccountEmail {
    email: String,
}
pub(super) async fn change_email(
    State(s): State<AppState>,
    Administrator(t): Administrator,
    Path(id): Path<String>,
    Json(input): Json<AccountEmail>,
) -> Result<Json<crate::models::Teacher>> {
    if id == t.id {
        // Changing one's own login requires the password; see /auth/me/email.
        return Err(Error(
            ErrorKind::Forbidden,
            "Die eigene E-Mail-Adresse wird unter „Mein Konto“ geändert.".into(),
        ));
    }
    let address = email(&input.email)?;
    let teacher = s.db.change_email(&t.id, &id, None, &address).await?;
    tracing::info!(target: "tamara::security", event="email_changed", actor_id=%t.id, account_id=%id);
    Ok(Json(teacher))
}

#[derive(Deserialize)]
pub(crate) struct UsageQuery {
    pub(crate) from: Option<String>,
    pub(crate) until: Option<String>,
    pub(crate) bucket: Option<String>,
}

/// School-wide daily totals only: no teacher, class or pupil data.
pub(super) async fn usage(
    State(s): State<AppState>,
    _admin: Administrator,
    Query(q): Query<UsageQuery>,
) -> Result<Json<Usage>> {
    let bucket = match q.bucket.as_deref().unwrap_or("day") {
        "day" => Bucket::Day,
        "week" => Bucket::Week,
        "month" => Bucket::Month,
        _ => return Err(bad("Bitte Tag, Woche oder Monat als Zeitraster wählen.")),
    };
    for date in [&q.from, &q.until].into_iter().flatten() {
        if !valid_date(date) {
            return Err(bad("Bitte gültige Daten im Format JJJJ-MM-TT angeben."));
        }
    }
    if let (Some(from), Some(until)) = (&q.from, &q.until)
        && from > until
    {
        return Err(bad("Der Zeitraum muss vor seinem Ende beginnen."));
    }
    // Nothing is counted before counting started or after today, which also
    // bounds the number of days in the response.
    let today = s.db.utc_today().await?;
    let until = q.until.unwrap_or_else(|| today.clone()).min(today);
    let from = q.from.unwrap_or_default();
    Ok(Json(s.db.usage(&from, &until, bucket).await?))
}

//! Cookie transport and request extractors; authentication policy lives in auth.
use crate::{
    auth::{Auth, Credentials, Session},
    error::{Result, internal},
    models::Teacher,
    state::AppState,
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) struct Authenticated(pub(super) Teacher);
pub(super) struct Administrator(pub(super) Teacher);

impl FromRequestParts<AppState> for Authenticated {
    type Rejection = crate::error::Error;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let token = cookie(&parts.headers);
        Ok(Self(
            state.auth.authenticate(&state.db, token.as_deref()).await?,
        ))
    }
}
impl FromRequestParts<AppState> for Administrator {
    type Rejection = crate::error::Error;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let Authenticated(teacher) = Authenticated::from_request_parts(parts, state).await?;
        Auth::require_admin(&teacher)?;
        Ok(Self(teacher))
    }
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

fn session_response(session: Session) -> Result<Response> {
    let mut response = Json(session.teacher).into_response();
    let cookie = format!(
        "tamara_session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800; Secure",
        session.token
    );
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(internal)?,
    );
    Ok(response)
}

pub(super) async fn register(
    State(s): State<AppState>,
    Json(c): Json<Credentials>,
) -> Result<Response> {
    let session = s.auth.register(&s.db, c).await?;
    tracing::info!(target: "tamara::security", event="account_registered", actor_id=%session.teacher.id);
    session_response(session)
}
pub(super) async fn login(
    State(s): State<AppState>,
    Json(c): Json<Credentials>,
) -> Result<Response> {
    let session = match s.auth.login(&s.db, c).await {
        Ok(session) => session,
        Err(error) => {
            tracing::warn!(target: "tamara::security", event="login_failed");
            return Err(error);
        }
    };
    tracing::info!(target: "tamara::security", event="login_succeeded", actor_id=%session.teacher.id);
    session_response(session)
}
pub(super) async fn auth_status(State(s): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(
        json!({"setup_required": s.db.teacher_count().await? == 0}),
    ))
}
pub(super) async fn me(Authenticated(teacher): Authenticated) -> Json<Teacher> {
    Json(teacher)
}
pub(super) async fn logout(State(s): State<AppState>, h: HeaderMap) -> Result<Response> {
    let token = cookie(&h);
    if let Ok(teacher) = s.auth.authenticate(&s.db, token.as_deref()).await {
        tracing::info!(target: "tamara::security", event="logout", actor_id=%teacher.id);
    }
    s.auth.logout(&s.db, token.as_deref()).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "tamara_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0; Secure",
        ),
    );
    Ok(response)
}

#[derive(Deserialize)]
pub(super) struct DeleteAccount {
    pub password: String,
    pub owned_classroom_ids: Vec<String>,
}
pub(super) async fn deletion_preview(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<crate::models::AccountDeletion>> {
    Ok(Json(s.db.account_deletion(&t.id).await?))
}
pub(super) async fn delete_account(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Json(input): Json<DeleteAccount>,
) -> Result<Response> {
    let proof = s.auth.confirm_password(&s.db, &t, input.password).await?;
    s.db.delete_account(&t.id, &t.id, &proof, &input.owned_classroom_ids)
        .await?;
    tracing::info!(target: "tamara::security", event="account_deleted", actor_id=%t.id, account_id=%t.id);
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "tamara_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0; Secure",
        ),
    );
    Ok(response)
}
#[derive(Deserialize)]
pub(super) struct ResetPassword {
    token: String,
    password: String,
}
pub(super) async fn reset_password(
    State(s): State<AppState>,
    Json(input): Json<ResetPassword>,
) -> Result<StatusCode> {
    let teacher = s
        .auth
        .reset_password(&s.db, &input.token, input.password)
        .await?;
    tracing::info!(target: "tamara::security", event="password_reset_completed", account_id=%teacher);
    Ok(StatusCode::NO_CONTENT)
}

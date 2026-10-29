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

fn session_response(session: Session, secure: bool) -> Result<Response> {
    let mut response = Json(session.teacher).into_response();
    let cookie = format!(
        "tamara_session={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=604800{}",
        session.token,
        if secure { "; Secure" } else { "" }
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
    session_response(s.auth.register(&s.db, c).await?, s.secure_cookie)
}
pub(super) async fn login(
    State(s): State<AppState>,
    Json(c): Json<Credentials>,
) -> Result<Response> {
    session_response(s.auth.login(&s.db, c).await?, s.secure_cookie)
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
    s.auth.logout(&s.db, cookie(&h).as_deref()).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("tamara_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
    );
    Ok(response)
}

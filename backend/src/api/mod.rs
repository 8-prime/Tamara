//! HTTP routing and transport. SQL and transactions stay in db.
mod admin;
mod assessments;
mod attendance;
mod auth;
mod classrooms;
mod conferences;
mod error;
mod exports;
mod homework;
mod notes;
mod parent_conferences;
mod pupil_history;
mod teaching;

use crate::{
    error::{Error, ErrorKind},
    state::AppState,
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use tower_http::services::{ServeDir, ServeFile};

async fn api_guard(req: axum::extract::Request, next: Next) -> Response {
    if !matches!(
        *req.method(),
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    ) && req
        .headers()
        .get("x-tamara-request")
        .and_then(|v| v.to_str().ok())
        != Some("1")
    {
        tracing::warn!(target: "tamara::security", event="mutation_guard_rejected");
        return Error(ErrorKind::Forbidden, "Ungültige Anfrage.".into()).into_response();
    }
    let method = req.method().clone();
    // Matched templates contain no user-supplied IDs, query strings or tokens.
    let route = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());
    let mut response = next.run(req).await;
    let status = response.status().as_u16();
    if matches!(status, 401 | 403 | 429) {
        tracing::warn!(target: "tamara::security", event="request_denied", %method, %route, status);
    } else if method != axum::http::Method::GET
        && method != axum::http::Method::HEAD
        && response.status().is_success()
    {
        tracing::info!(target: "tamara::security", event="mutation_completed", %method, %route, status);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

pub(crate) fn routes(s: AppState) -> Router {
    let dist = std::env::var("TAMARA_FRONTEND_DIR").unwrap_or_else(|_| "frontend/dist".into());
    routes_with_frontend(s, std::path::Path::new(&dist))
}

pub(crate) fn routes_with_frontend(s: AppState, dist: &std::path::Path) -> Router {
    let api = Router::new()
        .route(
            "/parent-conferences/pupils",
            get(parent_conferences::pupils),
        )
        .route("/parent-conferences", get(parent_conferences::list))
        .route(
            "/parent-conferences/{id}",
            axum::routing::put(parent_conferences::save).delete(parent_conferences::remove),
        )
        .route(
            "/parent-conferences/{id}/review",
            post(parent_conferences::capture).delete(parent_conferences::remove_review),
        )
        .route(
            "/parent-conferences/{id}/documents",
            get(parent_conferences::documents)
                .post(parent_conferences::upload)
                .layer(DefaultBodyLimit::max(parent_conferences::MAX_DOCUMENT)),
        )
        .route(
            "/parent-conferences/{id}/documents/{document}",
            get(parent_conferences::download).delete(parent_conferences::remove_document),
        )
        .route("/conferences", get(conferences::list))
        .route(
            "/conferences/{id}",
            axum::routing::put(conferences::save).delete(conferences::remove),
        )
        .route(
            "/classrooms/{class}/pupil-history",
            get(pupil_history::history),
        )
        .route("/auth/status", get(auth::auth_status))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me).delete(auth::delete_account))
        .route("/auth/me/deletion", get(auth::deletion_preview))
        .route("/auth/reset-password", post(auth::reset_password))
        .route("/admin/accounts", get(admin::accounts))
        .route(
            "/admin/accounts/{id}",
            patch(admin::set_role).delete(admin::delete_account),
        )
        .route(
            "/admin/accounts/{id}/deletion",
            get(admin::deletion_preview),
        )
        .route(
            "/admin/accounts/{id}/password-reset",
            post(admin::password_reset),
        )
        .route(
            "/admin/invitations",
            get(admin::invitations).post(admin::create_invitation),
        )
        .route("/admin/invitations/{id}", delete(admin::revoke_invitation))
        .route("/admin/backups", post(admin::create_backup))
        .route(
            "/classrooms",
            get(classrooms::classes).post(classrooms::create_class),
        )
        .route("/criteria", get(assessments::criteria))
        .route(
            "/lessons",
            get(teaching::lessons).post(teaching::create_lesson),
        )
        .route(
            "/lessons/{lesson}",
            get(teaching::lesson).patch(teaching::edit_lesson),
        )
        .route(
            "/lessons/{lesson}/performances",
            get(teaching::performances).patch(teaching::save_performance),
        )
        .route("/timetable/{entry}/cancel", post(teaching::cancel_series))
        .route("/tasks", get(teaching::all_tasks))
        .route(
            "/classrooms/{class}/tasks",
            get(teaching::tasks).post(teaching::create_task),
        )
        .route(
            "/classrooms/{class}/tasks/{task}",
            get(teaching::task)
                .patch(teaching::edit_task)
                .delete(teaching::delete_task),
        )
        .route(
            "/classrooms/{class}/tasks/{task}/completion",
            patch(teaching::complete_task),
        )
        .route("/classrooms/{class}/workspace", get(classrooms::workspace))
        .route(
            "/classrooms/{class}/notes",
            get(notes::list).post(notes::create),
        )
        .route(
            "/classrooms/{class}/notes/{note}",
            patch(notes::edit).delete(notes::remove),
        )
        .route(
            "/classrooms/{class}/subjects",
            get(homework::subjects).post(homework::create_subject),
        )
        .route(
            "/classrooms/{class}/subjects/{subject}",
            patch(homework::rename_subject).delete(homework::remove_subject),
        )
        .route(
            "/classrooms/{class}/homework",
            get(homework::list).post(homework::create),
        )
        .route(
            "/classrooms/{class}/homework/{homework}",
            get(homework::detail).patch(homework::edit),
        )
        .route(
            "/classrooms/{class}/homework/{homework}/submissions",
            patch(homework::save_submissions),
        )
        .route(
            "/classrooms/{class}/homework/{homework}/closure",
            patch(homework::set_closed),
        )
        .route(
            "/classrooms/{class}/homework/{homework}/worksheet",
            get(homework::worksheet)
                .put(homework::upload_worksheet)
                .delete(homework::remove_worksheet)
                .layer(DefaultBodyLimit::max(homework::MAX_WORKSHEET)),
        )
        .route(
            "/classrooms/{class}/attendance/{date}",
            get(attendance::list).patch(attendance::save),
        )
        .route(
            "/classrooms/{class}/students",
            post(classrooms::add_students),
        )
        .route(
            "/classrooms/{class}/students/{student}",
            patch(classrooms::rename_student).delete(classrooms::remove_student),
        )
        .route(
            "/classrooms/{class}/periods",
            post(assessments::create_period),
        )
        .route(
            "/classrooms/{class}/periods/{period}/assessments",
            patch(assessments::assess),
        )
        .route(
            "/classrooms/{class}/periods/{period}/zip",
            get(exports::export_zip),
        )
        .route("/classrooms/{class}/teachers", post(classrooms::share))
        .route(
            "/classrooms/{class}/teachers/{teacher}",
            delete(classrooms::unshare),
        )
        .route(
            "/classrooms/{class}/periods/{period}/students/{student}/pdf",
            get(exports::export_pdf),
        )
        .fallback(|| async { Error(ErrorKind::NotFound, "API-Endpunkt nicht gefunden.".into()) })
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn(api_guard));
    Router::new()
        .nest("/api", api)
        // Missing bundles must stay 404s rather than return the SPA's HTML.
        .nest_service("/assets", ServeDir::new(dist.join("assets")))
        .fallback_service(ServeDir::new(dist).fallback(ServeFile::new(dist.join("index.html"))))
        .with_state(s)
}

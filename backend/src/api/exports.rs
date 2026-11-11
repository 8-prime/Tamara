use super::auth::Authenticated;
use crate::{
    archive,
    db::{
        exports::{ClassExport, PdfExport},
        usage::Metric,
    },
    error::{Result, internal},
    pdf,
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderValue, header},
    response::Response,
};

pub(super) async fn export_pdf(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, period, student)): Path<(String, String, String)>,
) -> Result<Response> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.check_period(&class, &period).await?;
    let PdfExport {
        name,
        year,
        period_name,
        yes,
    } = s.db.pdf_export(&class, &period, &student).await?;
    let bytes = tokio::task::spawn_blocking(move || pdf::render(&name, &year, &period_name, &yes))
        .await
        .map_err(internal)?
        .map_err(internal)?;
    count_export(&s, Metric::PdfExported).await;
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"entwicklungsuebersicht-mathematik.pdf\""),
    );
    tracing::info!(target: "tamara::security", event="pupil_pdf_exported", actor_id=%t.id, classroom_id=%class);
    Ok(r)
}

pub(super) async fn export_zip(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, period)): Path<(String, String)>,
) -> Result<Response> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let ClassExport {
        students,
        year,
        period_name,
        marks,
    } = s.db.class_export(&class, &period).await?;
    let bytes =
        tokio::task::spawn_blocking(move || archive::render(students, year, period_name, marks))
            .await
            .map_err(internal)?
            .map_err(internal)?;
    count_export(&s, Metric::ZipExported).await;
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"mathematik-klasse.zip\""),
    );
    r.headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    tracing::info!(target: "tamara::security", event="classroom_zip_exported", actor_id=%t.id, classroom_id=%class);
    Ok(r)
}

/// Usage statistics must never block a download; a missed count is acceptable.
async fn count_export(s: &AppState, metric: Metric) {
    if let Err(error) = s.db.count_usage(metric).await {
        tracing::warn!(%error, "Could not count export");
    }
}

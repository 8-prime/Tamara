use super::auth::Authenticated;
use crate::{
    archive,
    db::exports::{ClassExport, PdfExport},
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
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    r.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"entwicklungsuebersicht-mathematik.pdf\""),
    );
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
    Ok(r)
}

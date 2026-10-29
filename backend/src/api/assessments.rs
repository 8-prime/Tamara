use super::auth::Authenticated;
use crate::{
    error::{Result, bad},
    models::AssessmentChange,
    pdf,
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
pub(crate) struct NewPeriod {
    pub(crate) name: String,
    pub(crate) source_period_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Changes {
    pub(crate) changes: Vec<AssessmentChange>,
}

pub(super) async fn create_period(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(p): Json<NewPeriod>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let name = text(&p.name, "Zeitraum", 120)?;
    let id = id();
    if let Some(ref source) = p.source_period_id {
        s.db.check_period(&class, source).await?;
    }
    s.db.create_period(&id, &class, &name, p.source_period_id.as_deref(), &t.id)
        .await?;
    Ok(Json(json!({"id":id})))
}

pub(super) async fn assess(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, period)): Path<(String, String)>,
    Json(c): Json<Changes>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.check_period(&class, &period).await?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad("Bitte 1 bis 5000 Bewertungen pro Anfrage senden."));
    }
    s.db.save_assessments(&class, &period, &t.id, c.changes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn criteria(_teacher: Authenticated) -> Json<&'static pdf::Catalog> {
    Json(pdf::catalog())
}

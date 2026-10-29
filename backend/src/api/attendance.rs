use super::auth::Authenticated;
use crate::{
    error::{Result, bad},
    models::{Attendance, AttendanceChange},
    state::AppState,
    validation::valid_date,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

fn check_date(date: &str) -> Result<()> {
    if !valid_date(date) {
        return Err(bad(
            "Bitte ein gültiges Datum im Format JJJJ-MM-TT angeben.",
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
pub(crate) struct AttendanceChanges {
    pub(crate) changes: Vec<AttendanceChange>,
}

pub(super) async fn list(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, date)): Path<(String, String)>,
) -> Result<Json<Vec<Attendance>>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    check_date(&date)?;
    let records = s.db.attendance(&class, &date).await?;
    Ok(Json(records))
}

pub(super) async fn save(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, date)): Path<(String, String)>,
    Json(c): Json<AttendanceChanges>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    check_date(&date)?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad(
            "Bitte 1 bis 5000 Anwesenheitseinträge pro Anfrage senden.",
        ));
    }
    s.db.save_attendance(&class, &date, &t.id, c.changes)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

use super::auth::Authenticated;
use crate::{
    error::{Result, bad},
    models::Classroom,
    state::AppState,
    util::id,
    validation::{email, text},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub(crate) struct NewClass {
    pub(crate) name: String,
    pub(crate) school_year: String,
}

#[derive(Deserialize)]
pub(crate) struct WorkspaceQuery {
    pub(crate) period_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct NewStudents {
    pub(crate) names: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct Name {
    pub(crate) name: String,
}

#[derive(Deserialize)]
pub(crate) struct Share {
    pub(crate) email: String,
}

pub(super) async fn classes(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<Vec<Classroom>>> {
    Ok(Json(s.db.classes(&t.id).await?))
}

pub(super) async fn create_class(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Json(c): Json<NewClass>,
) -> Result<Json<Value>> {
    let name = text(&c.name, "Klassenname", 80)?;
    let year = text(&c.school_year, "Schuljahr", 40)?;
    let id = id();
    s.db.create_class(&id, &name, &year, &t.id).await?;
    Ok(Json(json!({"id":id})))
}

pub(super) async fn workspace(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Query(q): Query<WorkspaceQuery>,
) -> Result<Json<Value>> {
    let role = s.auth.member(&s.db, &class, &t.id).await?;
    let students = s.db.students(&class).await?;
    let periods = s.db.periods(&class).await?;
    let period = q
        .period_id
        .or_else(|| periods.first().map(|p| p.id.clone()));
    let assessments = if let Some(ref p) = period {
        s.db.check_period(&class, p).await?;
        s.db.assessments(p).await?
    } else {
        vec![]
    };
    let teachers = s.db.collaborators(&class).await?;
    Ok(Json(
        json!({"students":students,"periods":periods,"period_id":period,"assessments":assessments,"teachers":teachers,"role":role}),
    ))
}

pub(super) async fn add_students(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(c): Json<NewStudents>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    if c.names.is_empty() || c.names.len() > 200 {
        return Err(bad("Bitte 1 bis 200 Namen eingeben."));
    }
    let names = c
        .names
        .iter()
        .map(|n| text(n, "Name", 120))
        .collect::<Result<Vec<_>>>()?;
    s.db.add_students(&class, names).await?;
    Ok(Json(json!({"added":c.names.len()})))
}

pub(super) async fn rename_student(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, student)): Path<(String, String)>,
    Json(n): Json<Name>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let name = text(&n.name, "Name", 120)?;
    s.db.rename_student(&class, &student, &name).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn remove_student(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, student)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.owner(&s.db, &class, &t.id).await?;
    s.db.remove_student(&class, &student).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn share(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(c): Json<Share>,
) -> Result<StatusCode> {
    s.auth.owner(&s.db, &class, &t.id).await?;
    s.db.share_class(&class, &email(&c.email)?).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn unshare(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, teacher)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.owner(&s.db, &class, &t.id).await?;
    if t.id == teacher {
        return Err(bad("Die Klassenleitung kann nicht entfernt werden."));
    }
    s.db.unshare_class(&class, &teacher).await?;
    Ok(StatusCode::NO_CONTENT)
}

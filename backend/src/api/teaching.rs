use super::auth::Authenticated;
use crate::{
    error::{Result, bad, missing},
    models::teaching::*,
    state::AppState,
    validation::{text, valid_date},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub(super) struct Range {
    from: String,
    until: String,
    classroom_id: Option<String>,
}

fn times(date: &str, start: &str, end: &str) -> Result<()> {
    fn valid(t: &str) -> bool {
        let b = t.as_bytes();
        b.len() == 5
            && b[2] == b':'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 2 || c.is_ascii_digit())
            && t[..2].parse::<u8>().is_ok_and(|v| v < 24)
            && t[3..].parse::<u8>().is_ok_and(|v| v < 60)
    }
    if !valid_date(date) || !valid(start) || !valid(end) || start >= end {
        return Err(bad(
            "Bitte ein gültiges Datum und eine Endzeit nach der Startzeit eingeben.",
        ));
    }
    Ok(())
}

fn paragraph(value: &str, max: usize) -> Result<String> {
    let s = value.trim();
    if s.chars().count() > max
        || s.chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(bad(&format!(
            "Text darf höchstens {max} Zeichen enthalten."
        )));
    }
    Ok(s.into())
}

pub(super) async fn lessons(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Query(q): Query<Range>,
) -> Result<Json<Vec<Lesson>>> {
    if !valid_date(&q.from) || !valid_date(&q.until) || q.from > q.until {
        return Err(bad("Ungültiger Zeitraum."));
    }
    if let Some(class) = &q.classroom_id {
        s.auth.member(&s.db, class, &t.id).await?;
    }
    Ok(Json(
        s.db.lessons(&t.id, &q.from, &q.until, q.classroom_id.as_deref())
            .await?,
    ))
}

pub(super) async fn lesson(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<Lesson>> {
    Ok(Json(s.db.lesson(&id, &t.id).await?))
}

pub(super) async fn create_lesson(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Json(mut c): Json<LessonInput>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &c.classroom_id, &t.id).await?;
    times(&c.lesson_date, &c.starts_at, &c.ends_at)?;
    if c.repeat_until
        .as_ref()
        .is_some_and(|v| !valid_date(v) || v < &c.lesson_date)
    {
        return Err(bad("Ungültiges Ende der Wiederholung."));
    }
    c.room = paragraph(&c.room, 80)?;
    c.topic = paragraph(&c.topic, 500)?;
    c.learning_goals = paragraph(&c.learning_goals, 5000)?;
    Ok(Json(json!({"id":s.db.create_lesson(&t.id,&c).await?})))
}

pub(super) async fn edit_lesson(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(mut c): Json<LessonEdit>,
) -> Result<StatusCode> {
    let lesson = s.db.lesson(&id, &t.id).await?;
    times(&c.lesson_date, &c.starts_at, &c.ends_at)?;
    if !["scheduled", "cancelled"].contains(&c.status.as_str()) {
        return Err(bad("Ungültiger Stundenstatus."));
    }
    c.room = paragraph(&c.room, 80)?;
    c.topic = paragraph(&c.topic, 500)?;
    c.learning_goals = paragraph(&c.learning_goals, 5000)?;
    s.db.edit_lesson(&lesson, &c).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(super) struct CancelSeries {
    from: String,
}
pub(super) async fn cancel_series(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(c): Json<CancelSeries>,
) -> Result<StatusCode> {
    if !valid_date(&c.from) {
        return Err(bad("Ungültiges Datum."));
    }
    // The cutoff must be a future calendar date in all supported school time zones.
    let today = s.db.utc_today().await?;
    if c.from <= today {
        return Err(bad(
            "Serien können erst ab einem zukünftigen Tag abgesagt werden. Einzelne Stunden bitte direkt bearbeiten.",
        ));
    }
    s.db.cancel_series(&id, &t.id, &c.from).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn performances(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
) -> Result<Json<Vec<Performance>>> {
    s.db.lesson(&id, &t.id).await?;
    Ok(Json(s.db.performances(&id).await?))
}
pub(super) async fn save_performance(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(id): Path<String>,
    Json(mut c): Json<Performance>,
) -> Result<StatusCode> {
    let lesson = s.db.lesson(&id, &t.id).await?;
    if !(1..=4).contains(&c.rating) {
        return Err(bad("Bitte eine Bewertung von 1 bis 4 wählen."));
    }
    c.observation = paragraph(&c.observation, 500)?;
    s.db.save_performance(&lesson, &t.id, &c).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn all_tasks(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
) -> Result<Json<Vec<ClassTask>>> {
    Ok(Json(s.db.tasks(&t.id, None).await?))
}
pub(super) async fn tasks(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
) -> Result<Json<Vec<ClassTask>>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    Ok(Json(s.db.tasks(&t.id, Some(&class)).await?))
}
pub(super) async fn task(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, id)): Path<(String, String)>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    let task =
        s.db.tasks(&t.id, Some(&class))
            .await?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(missing)?;
    let pupils = s.db.task_pupils(&class, &id).await?;
    Ok(Json(json!({"task":task,"pupils":pupils})))
}
fn validate_task(c: &mut TaskInput) -> Result<()> {
    c.title = text(&c.title, "Aufgabe", 200)?;
    c.description = paragraph(&c.description, 5000)?;
    if !["once", "per_pupil"].contains(&c.completion_mode.as_str())
        || c.due_date.as_ref().is_some_and(|v| !valid_date(v))
    {
        return Err(bad("Bitte Aufgabenart und Fälligkeitsdatum prüfen."));
    }
    Ok(())
}
pub(super) async fn create_task(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path(class): Path<String>,
    Json(mut c): Json<TaskInput>,
) -> Result<Json<Value>> {
    s.auth.member(&s.db, &class, &t.id).await?;
    validate_task(&mut c)?;
    Ok(Json(json!({"id":s.db.create_task(&class,&t.id,&c).await?})))
}
pub(super) async fn edit_task(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, id)): Path<(String, String)>,
    Json(mut c): Json<TaskInput>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    validate_task(&mut c)?;
    s.db.edit_task(&class, &id, &c).await?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
pub(super) struct Completion {
    student_id: Option<String>,
    completed: bool,
}
pub(super) async fn complete_task(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, id)): Path<(String, String)>,
    Json(c): Json<Completion>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.complete_task(&class, &id, c.student_id.as_deref(), c.completed, &t.id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn delete_task(
    State(s): State<AppState>,
    Authenticated(t): Authenticated,
    Path((class, id)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.auth.member(&s.db, &class, &t.id).await?;
    s.db.delete_task(&class, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

use super::*;

#[derive(Serialize, FromRow)]
pub(super) struct Note {
    id: String,
    student_id: String,
    content: String,
    created_by: String,
    created_at: i64,
    updated_by: String,
    updated_at: i64,
}

#[derive(Deserialize)]
pub(super) struct NewNote {
    student_id: String,
    content: String,
}

#[derive(Deserialize)]
pub(super) struct EditNote {
    content: String,
}

fn content(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 10000
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(bad("Bitte eine Notiz mit 1 bis 10000 Zeichen eingeben."));
    }
    Ok(value.into())
}

pub(super) async fn list(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
) -> Result<Json<Vec<Note>>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let notes = sqlx::query_as::<_, Note>(
        "SELECT id,student_id,content,created_by,created_at,updated_by,updated_at FROM student_notes WHERE classroom_id=? ORDER BY created_at DESC,rowid DESC",
    )
    .bind(class)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(notes))
}

pub(super) async fn create(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(class): Path<String>,
    Json(input): Json<NewNote>,
) -> Result<Json<Note>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let content = content(&input.content)?;
    // The insert checks class membership of the child in the same statement.
    let note = sqlx::query_as::<_, Note>(
        "INSERT INTO student_notes(id,classroom_id,student_id,content,created_by,updated_by) SELECT ?,?,?,?,?,? WHERE EXISTS(SELECT 1 FROM students WHERE id=? AND classroom_id=?) RETURNING id,student_id,content,created_by,created_at,updated_by,updated_at",
    )
    .bind(id())
    .bind(&class)
    .bind(&input.student_id)
    .bind(content)
    .bind(&t.id)
    .bind(&t.id)
    .bind(&input.student_id)
    .bind(&class)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)?;
    Ok(Json(note))
}

pub(super) async fn edit(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, note)): Path<(String, String)>,
    Json(input): Json<EditNote>,
) -> Result<Json<Note>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    let content = content(&input.content)?;
    let note = sqlx::query_as::<_, Note>(
        "UPDATE student_notes SET content=?,updated_by=?,updated_at=unixepoch() WHERE id=? AND classroom_id=? RETURNING id,student_id,content,created_by,created_at,updated_by,updated_at",
    )
    .bind(content)
    .bind(t.id)
    .bind(note)
    .bind(class)
    .fetch_optional(&s.db)
    .await?
    .ok_or_else(missing)?;
    Ok(Json(note))
}

pub(super) async fn remove(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, note)): Path<(String, String)>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    if sqlx::query("DELETE FROM student_notes WHERE id=? AND classroom_id=?")
        .bind(note)
        .bind(class)
        .execute(&s.db)
        .await?
        .rows_affected()
        == 0
    {
        return Err(missing());
    }
    Ok(StatusCode::NO_CONTENT)
}

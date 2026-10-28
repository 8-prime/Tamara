use super::*;

#[derive(Serialize, FromRow)]
pub(super) struct Attendance {
    student_id: String,
    status: String,
    updated_by: String,
    updated_at: i64,
}

fn check_date(date: &str) -> Result<()> {
    if !valid_date(date) {
        return Err(bad(
            "Bitte ein gültiges Datum im Format JJJJ-MM-TT angeben.",
        ));
    }
    Ok(())
}

pub(super) async fn list(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, date)): Path<(String, String)>,
) -> Result<Json<Vec<Attendance>>> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    check_date(&date)?;
    let records = sqlx::query_as::<_, Attendance>(
        "SELECT student_id,status,updated_by,updated_at FROM attendance WHERE classroom_id=? AND attendance_date=? ORDER BY student_id",
    )
    .bind(class)
    .bind(date)
    .fetch_all(&s.db)
    .await?;
    Ok(Json(records))
}

#[derive(Deserialize)]
struct AttendanceChange {
    student_id: String,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct AttendanceChanges {
    changes: Vec<AttendanceChange>,
}

pub(super) async fn save(
    State(s): State<AppState>,
    h: HeaderMap,
    Path((class, date)): Path<(String, String)>,
    Json(c): Json<AttendanceChanges>,
) -> Result<StatusCode> {
    let t = auth(&s, &h).await?;
    member(&s, &class, &t.id).await?;
    check_date(&date)?;
    if c.changes.is_empty() || c.changes.len() > 5000 {
        return Err(bad(
            "Bitte 1 bis 5000 Anwesenheitseinträge pro Anfrage senden.",
        ));
    }
    let mut tx = s.db.begin_with("BEGIN IMMEDIATE").await?;
    let students: HashSet<String> =
        sqlx::query_scalar("SELECT id FROM students WHERE classroom_id=?")
            .bind(&class)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect();
    let mut seen = HashSet::new();
    for ch in &c.changes {
        if !students.contains(&ch.student_id) || !seen.insert(&ch.student_id) {
            return Err(bad("Unbekanntes oder mehrfach angegebenes Kind."));
        }
        if ch
            .status
            .as_deref()
            .is_some_and(|v| v != "present" && v != "absent")
        {
            return Err(bad("Die Anwesenheit muss present, absent oder null sein."));
        }
    }
    for ch in c.changes {
        if let Some(status) = ch.status {
            sqlx::query("INSERT INTO attendance(classroom_id,student_id,attendance_date,status,updated_by) VALUES(?,?,?,?,?) ON CONFLICT(classroom_id,attendance_date,student_id) DO UPDATE SET status=excluded.status,updated_by=excluded.updated_by,updated_at=unixepoch()")
                .bind(&class).bind(ch.student_id).bind(&date).bind(status).bind(&t.id)
                .execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM attendance WHERE classroom_id=? AND attendance_date=? AND student_id=?")
                .bind(&class).bind(&date).bind(ch.student_id).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

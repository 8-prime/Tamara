use super::Database;
use crate::{
    error::{Result, bad, missing},
    models::teaching::{
        ClassTask, Lesson, LessonEdit, LessonInput, Performance, TaskInput, TaskPupil,
    },
    util::id,
};

const LESSONS: &str = "SELECT l.id,l.timetable_entry_id,l.teacher_id,l.classroom_id,c.name class_name,
 l.subject_id,s.name subject_name,l.lesson_date,l.starts_at,l.ends_at,l.room,l.topic,l.learning_goals,l.status,
 (SELECT COUNT(*) FROM students WHERE classroom_id=l.classroom_id) pupil_count,
 (SELECT COUNT(*) FROM attendance WHERE classroom_id=l.classroom_id AND attendance_date=l.lesson_date) attendance_count,
 (SELECT COUNT(*) FROM attendance WHERE classroom_id=l.classroom_id AND attendance_date=l.lesson_date AND status='present') present_count,
 (SELECT COUNT(*) FROM lesson_performances WHERE lesson_id=l.id) performance_count
 FROM lessons l JOIN classrooms c ON c.id=l.classroom_id JOIN subjects s ON s.id=l.subject_id";

const TASKS: &str = "SELECT t.id,t.classroom_id,c.name class_name,t.lesson_id,t.title,t.description,
 t.completion_mode,t.due_date,t.completed_at,
 (SELECT COUNT(*) FROM class_task_pupils WHERE task_id=t.id) assigned_count,
 (SELECT COUNT(*) FROM class_task_pupils WHERE task_id=t.id AND completed_at IS NOT NULL) completed_count
 FROM class_tasks t JOIN classrooms c ON c.id=t.classroom_id";

impl Database {
    pub(crate) async fn utc_today(&self) -> Result<String> {
        Ok(sqlx::query_scalar("SELECT date('now')")
            .fetch_one(&self.pool)
            .await?)
    }
    pub(crate) async fn lessons(
        &self,
        teacher: &str,
        from: &str,
        until: &str,
        class: Option<&str>,
    ) -> Result<Vec<Lesson>> {
        Ok(sqlx::query_as(&format!("{LESSONS} WHERE l.lesson_date BETWEEN ? AND ?
          AND EXISTS(SELECT 1 FROM classroom_teachers WHERE classroom_id=l.classroom_id AND teacher_id=?)
          AND ((? IS NULL AND l.teacher_id=?) OR l.classroom_id=?) ORDER BY l.lesson_date,l.starts_at,l.id"))
          .bind(from).bind(until).bind(teacher).bind(class).bind(teacher).bind(class)
          .fetch_all(&self.pool).await?)
    }

    pub(crate) async fn lesson(&self, lesson: &str, teacher: &str) -> Result<Lesson> {
        sqlx::query_as(&format!(
            "{LESSONS} WHERE l.id=? AND EXISTS(
          SELECT 1 FROM classroom_teachers WHERE classroom_id=l.classroom_id AND teacher_id=?)"
        ))
        .bind(lesson)
        .bind(teacher)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(missing)
    }

    pub(crate) async fn create_lesson(&self, teacher: &str, input: &LessonInput) -> Result<String> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let valid: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM subjects WHERE id=? AND classroom_id=?")
                .bind(&input.subject_id)
                .bind(&input.classroom_id)
                .fetch_one(&mut *tx)
                .await?;
        if valid != 1 {
            return Err(bad("Bitte ein Fach dieser Klasse auswählen."));
        }
        let until = input.repeat_until.as_deref().unwrap_or(&input.lesson_date);
        let span: i64 = sqlx::query_scalar("SELECT CAST(julianday(?)-julianday(?) AS INTEGER)")
            .bind(until)
            .bind(&input.lesson_date)
            .fetch_one(&mut *tx)
            .await?;
        if !(0..=366).contains(&span) {
            return Err(bad("Eine Wiederholung darf höchstens ein Jahr umfassen."));
        }
        let dates: Vec<String> = sqlx::query_scalar("WITH RECURSIVE dates(d) AS (
          SELECT ? UNION ALL SELECT date(d,'+7 days') FROM dates WHERE date(d,'+7 days')<=?) SELECT d FROM dates")
          .bind(&input.lesson_date).bind(until).fetch_all(&mut *tx).await?;
        let entry = if input.repeat_until.is_some() {
            let entry = id();
            sqlx::query("INSERT INTO timetable_entries(id,teacher_id,classroom_id,subject_id,weekday,starts_at,ends_at,room,valid_from,valid_until)
              VALUES(?,?,?,?,CAST(strftime('%w',?) AS INTEGER),?,?,?,?,?)")
              .bind(&entry).bind(teacher).bind(&input.classroom_id).bind(&input.subject_id).bind(&input.lesson_date)
              .bind(&input.starts_at).bind(&input.ends_at).bind(&input.room).bind(&input.lesson_date).bind(until)
              .execute(&mut *tx).await?;
            Some(entry)
        } else {
            None
        };
        let first = id();
        for (index, date) in dates.iter().enumerate() {
            let overlap: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM lessons WHERE teacher_id=? AND lesson_date=?
              AND status='scheduled' AND starts_at<? AND ends_at>?",
            )
            .bind(teacher)
            .bind(date)
            .bind(&input.ends_at)
            .bind(&input.starts_at)
            .fetch_one(&mut *tx)
            .await?;
            if overlap > 0 {
                return Err(bad(&format!(
                    "Am {date} überschneidet sich diese Stunde mit einem bestehenden Eintrag."
                )));
            }
            sqlx::query("INSERT INTO lessons(id,timetable_entry_id,teacher_id,classroom_id,subject_id,scheduled_date,lesson_date,starts_at,ends_at,room,topic,learning_goals)
              VALUES(?,?,?,?,?,?,?,?,?,?,?,?)")
              .bind(if index == 0 { first.clone() } else { id() }).bind(&entry).bind(teacher)
              .bind(&input.classroom_id).bind(&input.subject_id).bind(date).bind(date)
              .bind(&input.starts_at).bind(&input.ends_at).bind(&input.room).bind(&input.topic).bind(&input.learning_goals)
              .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(first)
    }

    pub(crate) async fn edit_lesson(&self, lesson: &Lesson, input: &LessonEdit) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if input.status == "scheduled" {
            let overlap: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM lessons WHERE teacher_id=? AND lesson_date=?
              AND id<>? AND status='scheduled' AND starts_at<? AND ends_at>?",
            )
            .bind(&lesson.teacher_id)
            .bind(&input.lesson_date)
            .bind(&lesson.id)
            .bind(&input.ends_at)
            .bind(&input.starts_at)
            .fetch_one(&mut *tx)
            .await?;
            if overlap > 0 {
                return Err(bad(
                    "Die Stunde überschneidet sich mit einem bestehenden Eintrag.",
                ));
            }
        }
        sqlx::query("UPDATE lessons SET lesson_date=?,starts_at=?,ends_at=?,room=?,topic=?,learning_goals=?,status=? WHERE id=?")
          .bind(&input.lesson_date).bind(&input.starts_at).bind(&input.ends_at).bind(&input.room)
          .bind(&input.topic).bind(&input.learning_goals).bind(&input.status).bind(&lesson.id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn cancel_series(&self, entry: &str, teacher: &str, from: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let accessible: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM timetable_entries e
          JOIN classroom_teachers m ON m.classroom_id=e.classroom_id AND m.teacher_id=? WHERE e.id=? AND e.teacher_id=?")
          .bind(teacher).bind(entry).bind(teacher).fetch_one(&mut *tx).await?;
        if accessible != 1 {
            return Err(missing());
        }
        sqlx::query(
            "UPDATE lessons SET status='cancelled' WHERE timetable_entry_id=? AND lesson_date>=?
          AND NOT EXISTS(SELECT 1 FROM lesson_performances WHERE lesson_id=lessons.id)
          AND NOT EXISTS(SELECT 1 FROM student_notes WHERE lesson_id=lessons.id)",
        )
        .bind(entry)
        .bind(from)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn performances(&self, lesson: &str) -> Result<Vec<Performance>> {
        Ok(sqlx::query_as("SELECT student_id,rating,observation FROM lesson_performances WHERE lesson_id=? ORDER BY student_id")
          .bind(lesson).fetch_all(&self.pool).await?)
    }

    pub(crate) async fn save_performance(
        &self,
        lesson: &Lesson,
        teacher: &str,
        input: &Performance,
    ) -> Result<()> {
        let saved = sqlx::query("INSERT INTO lesson_performances(lesson_id,classroom_id,student_id,rating,observation,updated_by)
          SELECT ?,?,?,?,?,? WHERE EXISTS(SELECT 1 FROM students WHERE id=? AND classroom_id=?)
          AND EXISTS(SELECT 1 FROM lessons WHERE id=? AND status='scheduled')
          ON CONFLICT(lesson_id,student_id) DO UPDATE SET rating=excluded.rating,observation=excluded.observation,
          updated_by=excluded.updated_by,updated_at=unixepoch()")
          .bind(&lesson.id).bind(&lesson.classroom_id).bind(&input.student_id).bind(input.rating).bind(&input.observation).bind(teacher)
          .bind(&input.student_id).bind(&lesson.classroom_id).bind(&lesson.id).execute(&self.pool).await?;
        if saved.rows_affected() != 1 {
            return Err(bad("Kind oder aktive Stunde nicht gefunden."));
        }
        Ok(())
    }

    pub(crate) async fn tasks(&self, teacher: &str, class: Option<&str>) -> Result<Vec<ClassTask>> {
        Ok(sqlx::query_as(&format!(
            "{TASKS} WHERE (? IS NULL OR t.classroom_id=?) AND EXISTS(
          SELECT 1 FROM classroom_teachers WHERE classroom_id=t.classroom_id AND teacher_id=?)
          ORDER BY t.due_date IS NULL,t.due_date,t.created_at,t.id"
        ))
        .bind(class)
        .bind(class)
        .bind(teacher)
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn task_pupils(&self, class: &str, task: &str) -> Result<Vec<TaskPupil>> {
        Ok(sqlx::query_as("SELECT p.student_id,s.name,p.completed_at FROM class_task_pupils p JOIN students s ON s.id=p.student_id
          WHERE p.classroom_id=? AND p.task_id=? ORDER BY s.name COLLATE NOCASE,s.id")
          .bind(class).bind(task).fetch_all(&self.pool).await?)
    }

    pub(crate) async fn create_task(
        &self,
        class: &str,
        teacher: &str,
        input: &TaskInput,
    ) -> Result<String> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        if let Some(lesson) = &input.lesson_id {
            let valid: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM lessons WHERE id=? AND classroom_id=?")
                    .bind(lesson)
                    .bind(class)
                    .fetch_one(&mut *tx)
                    .await?;
            if valid != 1 {
                return Err(missing());
            }
        }
        let task = id();
        sqlx::query("INSERT INTO class_tasks(id,classroom_id,lesson_id,title,description,completion_mode,due_date,created_by) VALUES(?,?,?,?,?,?,?,?)")
          .bind(&task).bind(class).bind(&input.lesson_id).bind(&input.title).bind(&input.description)
          .bind(&input.completion_mode).bind(&input.due_date).bind(teacher).execute(&mut *tx).await?;
        if input.completion_mode == "per_pupil" {
            let assigned = sqlx::query("INSERT INTO class_task_pupils(task_id,classroom_id,student_id) SELECT ?,classroom_id,id FROM students WHERE classroom_id=?")
              .bind(&task).bind(class).execute(&mut *tx).await?;
            if assigned.rows_affected() == 0 {
                return Err(bad("Bitte zuerst Kinder zur Klasse hinzufügen."));
            }
        }
        tx.commit().await?;
        Ok(task)
    }

    pub(crate) async fn edit_task(&self, class: &str, task: &str, input: &TaskInput) -> Result<()> {
        // Assignment mode and lesson are stable so editing cannot silently discard completions.
        let changed = sqlx::query("UPDATE class_tasks SET title=?,description=?,due_date=? WHERE id=? AND classroom_id=? AND completion_mode=?
          AND lesson_id IS ?")
          .bind(&input.title).bind(&input.description).bind(&input.due_date).bind(task).bind(class)
          .bind(&input.completion_mode).bind(&input.lesson_id).execute(&self.pool).await?;
        if changed.rows_affected() != 1 {
            return Err(bad(
                "Aufgabe nicht gefunden. Aufgabenart und Stundenbezug können nicht geändert werden.",
            ));
        }
        Ok(())
    }

    pub(crate) async fn complete_task(
        &self,
        class: &str,
        task: &str,
        student: Option<&str>,
        completed: bool,
        teacher: &str,
    ) -> Result<()> {
        let changed = if let Some(student) = student {
            sqlx::query("UPDATE class_task_pupils SET completed_at=CASE WHEN ? THEN unixepoch() ELSE NULL END,updated_by=?
              WHERE classroom_id=? AND task_id=? AND student_id=?")
              .bind(completed).bind(teacher).bind(class).bind(task).bind(student).execute(&self.pool).await?
        } else {
            sqlx::query(
                "UPDATE class_tasks SET completed_at=CASE WHEN ? THEN unixepoch() ELSE NULL END
              WHERE classroom_id=? AND id=? AND completion_mode='once'",
            )
            .bind(completed)
            .bind(class)
            .bind(task)
            .execute(&self.pool)
            .await?
        };
        if changed.rows_affected() != 1 {
            return Err(missing());
        }
        Ok(())
    }

    pub(crate) async fn delete_task(&self, class: &str, task: &str) -> Result<()> {
        if sqlx::query("DELETE FROM class_tasks WHERE classroom_id=? AND id=?")
            .bind(class)
            .bind(task)
            .execute(&self.pool)
            .await?
            .rows_affected()
            != 1
        {
            return Err(missing());
        }
        Ok(())
    }
}

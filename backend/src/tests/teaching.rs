use super::*;

async fn setup() -> (Client, SqlitePool, String, String, String) {
    let (app, db) = fixture().await;
    let mut owner = Client::new(app);
    owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "teaching@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let class = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"2a","school_year":"2026/27"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .call(
            &format!("/classrooms/{class}/students"),
            "POST",
            json!({"names":["Same Name","Same Name"]}),
            200,
        )
        .await;
    let student = owner.get(&format!("/classrooms/{class}/workspace")).await["students"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let subject = owner
        .call(
            &format!("/classrooms/{class}/subjects"),
            "POST",
            json!({"name":"Mathematik"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (owner, db, class, subject, student)
}
fn lesson_input(class: &str, subject: &str) -> Value {
    json!({"classroom_id":class,"subject_id":subject,"lesson_date":"2026-10-29","starts_at":"08:00","ends_at":"08:45"})
}

#[tokio::test]
async fn lessons_repeat_atomically_and_shared_access_is_enforced() {
    let (mut owner, db, class, subject, _) = setup().await;
    let mut input = lesson_input(&class, &subject);
    input["repeat_until"] = json!("2026-11-12");
    let first = owner.call("/lessons", "POST", input.clone(), 200).await;
    let list = owner.get("/lessons?from=2026-10-01&until=2026-12-01").await;
    assert_eq!(list.as_array().unwrap().len(), 3);
    assert_eq!(list[0]["id"], first["id"]);
    assert_eq!(list[1]["lesson_date"], "2026-11-05");
    // Two new occurrences can be inserted before a later collision: all must roll back.
    let mut later_conflict = lesson_input(&class, &subject);
    later_conflict["lesson_date"] = json!("2026-10-15");
    later_conflict["repeat_until"] = json!("2026-11-12");
    owner.call("/lessons", "POST", later_conflict, 400).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM lessons")
            .fetch_one(&db)
            .await
            .unwrap(),
        3
    );
    owner.call("/lessons", "POST", input.clone(), 400).await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM timetable_entries")
            .fetch_one(&db)
            .await
            .unwrap(),
        1
    );
    input["starts_at"] = json!("09:00");
    input["ends_at"] = json!("09:45");
    input["subject_id"] = json!("unknown");
    owner.call("/lessons", "POST", input.clone(), 400).await;
    input["subject_id"] = json!(subject);
    for (field, value) in [
        ("lesson_date", "2026-02-30"),
        ("starts_at", "25:00"),
        ("ends_at", "08:00"),
        ("repeat_until", "2028-01-01"),
    ] {
        let mut invalid = input.clone();
        invalid[field] = json!(value);
        owner.call("/lessons", "POST", invalid, 400).await;
    }
    let path = format!("/lessons/{}", first["id"].as_str().unwrap());
    owner.call(&path,"PATCH",json!({"lesson_date":"2026-10-30","starts_at":"10:00","ends_at":"10:45","topic":"Addition","learning_goals":"Zahlen zerlegen","status":"scheduled"}),204).await;
    let list = owner.get("/lessons?from=2026-10-01&until=2026-12-01").await;
    assert_eq!(list[0]["lesson_date"], "2026-10-30");
    assert_eq!(list[1]["lesson_date"], "2026-11-05");
    owner
        .call(
            &format!("/classrooms/{class}/subjects/{subject}"),
            "DELETE",
            Value::Null,
            409,
        )
        .await;
    let invite = owner
        .call("/admin/invitations", "POST", json!({"label":"Editor"}), 200)
        .await;
    let mut editor = Client::new(owner.app.clone());
    let teacher = editor
        .call(
            "/auth/register",
            "POST",
            registration(
                "lesson-editor@example.test",
                invite["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    editor.call(&path, "GET", Value::Null, 404).await;
    owner
        .call(
            &format!("/classrooms/{class}/teachers"),
            "POST",
            json!({"email":"lesson-editor@example.test"}),
            204,
        )
        .await;
    assert_eq!(
        editor
            .get("/lessons?from=2026-10-01&until=2026-12-01")
            .await,
        json!([])
    );
    assert_eq!(
        editor
            .get(&format!(
                "/lessons?from=2026-10-01&until=2026-12-01&classroom_id={class}"
            ))
            .await
            .as_array()
            .unwrap()
            .len(),
        3
    );
    editor.call(&path, "GET", Value::Null, 200).await;
    owner
        .call(
            &format!(
                "/classrooms/{class}/teachers/{}",
                teacher["id"].as_str().unwrap()
            ),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    editor.call(&path, "GET", Value::Null, 404).await;
    assert_eq!(
        editor
            .get("/lessons?from=2026-10-01&until=2026-12-01")
            .await,
        json!([])
    );
}

#[tokio::test]
async fn performances_and_notes_require_children_from_the_lessons_class() {
    let (mut owner, db, class, subject, student) = setup().await;
    let id = owner
        .call("/lessons", "POST", lesson_input(&class, &subject), 200)
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let path = format!("/lessons/{id}/performances");
    owner
        .call(
            &path,
            "PATCH",
            json!({"student_id":student,"rating":3,"observation":"Gut erklärt"}),
            204,
        )
        .await;
    owner
        .call(
            &path,
            "PATCH",
            json!({"student_id":student,"rating":4}),
            204,
        )
        .await;
    assert_eq!(
        owner.get(&path).await,
        json!([{"student_id":student,"rating":4,"observation":""}])
    );
    owner
        .call(
            &path,
            "PATCH",
            json!({"student_id":student,"rating":5}),
            400,
        )
        .await;
    owner
        .call(
            &path,
            "PATCH",
            json!({"student_id":"foreign","rating":3}),
            400,
        )
        .await;
    let notes = format!("/classrooms/{class}/notes");
    let note = owner
        .call(
            &notes,
            "POST",
            json!({"student_id":student,"content":"Beobachtung","lesson_id":id}),
            200,
        )
        .await;
    assert_eq!(note["lesson_id"], id);
    owner
        .call(
            &notes,
            "POST",
            json!({"student_id":student,"content":"Beobachtung","lesson_id":"foreign"}),
            404,
        )
        .await;
    let standalone = owner
        .call(
            &notes,
            "POST",
            json!({"student_id":student,"content":"Allgemein"}),
            200,
        )
        .await;
    assert!(standalone["lesson_id"].is_null());
    owner.call(&format!("/lessons/{id}"),"PATCH",json!({"lesson_date":"2026-10-29","starts_at":"08:00","ends_at":"08:45","status":"cancelled"}),204).await;
    owner
        .call(
            &path,
            "PATCH",
            json!({"student_id":student,"rating":2}),
            400,
        )
        .await;
    assert_eq!(owner.get(&path).await[0]["rating"], 4);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM math_assessments")
            .fetch_one(&db)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn class_tasks_snapshot_pupils_and_keep_once_completion_separate() {
    let (mut owner, _, class, subject, student) = setup().await;
    let endpoint = format!("/classrooms/{class}/tasks");
    let lesson = owner
        .call("/lessons", "POST", lesson_input(&class, &subject), 200)
        .await;
    let id=owner.call(&endpoint,"POST",json!({"title":"Erlaubnis einsammeln","completion_mode":"per_pupil","due_date":"2026-10-30","lesson_id":lesson["id"]}),200).await["id"].as_str().unwrap().to_owned();
    let path = format!("{endpoint}/{id}");
    owner
        .call(
            &format!("/classrooms/{class}/students"),
            "POST",
            json!({"names":["Added later"]}),
            200,
        )
        .await;
    assert_eq!(
        owner.get(&path).await["pupils"].as_array().unwrap().len(),
        2
    );
    owner
        .call(
            &format!("{path}/completion"),
            "PATCH",
            json!({"completed":true}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{path}/completion"),
            "PATCH",
            json!({"student_id":"foreign","completed":true}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{path}/completion"),
            "PATCH",
            json!({"student_id":student,"completed":true}),
            204,
        )
        .await;
    assert_eq!(owner.get(&path).await["task"]["completed_count"], 1);
    owner
        .call(
            &format!("{path}/completion"),
            "PATCH",
            json!({"student_id":student,"completed":false}),
            204,
        )
        .await;
    assert_eq!(owner.get(&path).await["task"]["completed_count"], 0);
    let once = owner
        .call(
            &endpoint,
            "POST",
            json!({"title":"Material vorbereiten","completion_mode":"once"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .call(
            &format!("{endpoint}/{once}/completion"),
            "PATCH",
            json!({"completed":true}),
            204,
        )
        .await;
    assert!(owner.get(&format!("{endpoint}/{once}")).await["task"]["completed_at"].is_i64());
    owner
        .call(
            &path,
            "PATCH",
            json!({"title":"Andere Art","completion_mode":"once"}),
            400,
        )
        .await;
    owner.call(&path, "DELETE", Value::Null, 204).await;
    assert_eq!(owner.get(&endpoint).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn teaching_migration_preserves_existing_records() {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    for migration in [
        include_str!("../../migrations/0001_initial.sql"),
        include_str!("../../migrations/0002_registration_tokens.sql"),
        include_str!("../../migrations/0003_achievement_dates.sql"),
        include_str!("../../migrations/0004_attendance.sql"),
        include_str!("../../migrations/0005_homework.sql"),
        include_str!("../../migrations/0006_notes.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&db).await.unwrap();
    }
    sqlx::raw_sql("INSERT INTO teachers(id,name,email,password_hash) VALUES('t','Teacher','test@example.test','hash');
      INSERT INTO classrooms(id,name,school_year) VALUES('c','2a','2026/27');
      INSERT INTO students(id,classroom_id,name) VALUES('p','c','Child');
      INSERT INTO subjects(id,classroom_id,name) VALUES('s','c','Deutsch');
      INSERT INTO homework(id,classroom_id,subject_id,title,due_date,created_by) VALUES('h','c','s','Lesen','2026-10-29','t');
      INSERT INTO homework_submissions(homework_id,classroom_id,student_id,handed_in_at) VALUES('h','c','p',123);
      INSERT INTO homework_worksheets(homework_id,name,content) VALUES('h','blatt.pdf',X'010203');
      INSERT INTO attendance(classroom_id,student_id,attendance_date,status,updated_by) VALUES('c','p','2026-10-29','present','t');
      INSERT INTO student_notes(id,classroom_id,student_id,content,created_by,updated_by) VALUES('n','c','p','Original note','t','t');")
      .execute(&db).await.unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0007_teaching.sql"))
        .execute(&db)
        .await
        .unwrap();
    let note: (String, Option<String>) =
        sqlx::query_as("SELECT content,lesson_id FROM student_notes")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(note, ("Original note".into(), None));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT handed_in_at FROM homework_submissions")
            .fetch_one(&db)
            .await
            .unwrap(),
        123
    );
    assert_eq!(
        sqlx::query_scalar::<_, Vec<u8>>("SELECT content FROM homework_worksheets")
            .fetch_one(&db)
            .await
            .unwrap(),
        vec![1, 2, 3]
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM attendance")
            .fetch_one(&db)
            .await
            .unwrap(),
        "present"
    );
    assert!(
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_all(&db)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn cancelling_a_series_preserves_lessons_with_captured_work() {
    let (mut owner, _, class, subject, student) = setup().await;
    let mut input = lesson_input(&class, &subject);
    input["lesson_date"] = json!("2099-01-01");
    input["repeat_until"] = json!("2099-01-15");
    owner.call("/lessons", "POST", input, 200).await;
    let list = owner.get("/lessons?from=2099-01-01&until=2099-01-31").await;
    owner
        .call(
            &format!("/classrooms/{class}/notes"),
            "POST",
            json!({"student_id":student,"content":"Vorbereitung","lesson_id":list[0]["id"]}),
            200,
        )
        .await;
    owner
        .call(
            &format!("/lessons/{}/performances", list[1]["id"].as_str().unwrap()),
            "PATCH",
            json!({"student_id":student,"rating":2}),
            204,
        )
        .await;
    let cancel = format!(
        "/timetable/{}/cancel",
        list[0]["timetable_entry_id"].as_str().unwrap()
    );
    owner
        .call(&cancel, "POST", json!({"from":"2000-01-01"}), 400)
        .await;
    owner
        .call(&cancel, "POST", json!({"from":"2099-01-01"}), 204)
        .await;
    let list = owner.get("/lessons?from=2099-01-01&until=2099-01-31").await;
    assert_eq!(list[0]["status"], "scheduled");
    assert_eq!(list[1]["status"], "scheduled");
    assert_eq!(list[2]["status"], "cancelled");
}

use super::*;

const SETUP_TOKEN: &str = "fixture-setup-token-at-least-24-characters";

async fn totals(admin: &mut Client) -> Value {
    admin.get("/admin/usage").await["totals"].clone()
}

#[tokio::test]
async fn usage_counts_school_wide_actions_without_identifying_data() {
    let (app, db) = fixture().await;
    let mut anonymous = Client::new(app.clone());
    anonymous
        .call("/admin/usage", "GET", Value::Null, 401)
        .await;
    let mut admin = Client::new(app.clone());
    admin
        .call(
            "/auth/register",
            "POST",
            registration("usage-admin@example.test", SETUP_TOKEN),
            200,
        )
        .await;
    let invitation = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Usage teacher"}),
            200,
        )
        .await;
    let mut teacher = Client::new(app.clone());
    teacher
        .call(
            "/auth/register",
            "POST",
            registration(
                "usage-teacher@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    teacher.call("/admin/usage", "GET", Value::Null, 403).await;

    let class = admin
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Geheime Klasse 4b","school_year":"2026/27"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let prefix = format!("/classrooms/{class}");
    admin
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Erika Mustermann","Max Mustermann"]}),
            200,
        )
        .await;
    let workspace = admin.get(&format!("{prefix}/workspace")).await;
    let (s0, s1) = (
        &workspace["students"][0]["id"],
        &workspace["students"][1]["id"],
    );

    let note = admin
        .call(
            &format!("{prefix}/notes"),
            "POST",
            json!({"student_id":s0,"content":"Vertrauliche Beobachtung"}),
            200,
        )
        .await;
    let note_path = format!("{prefix}/notes/{}", note["id"].as_str().unwrap());
    admin
        .call(
            &note_path,
            "PATCH",
            json!({"content":"Vertrauliche Beobachtung, ergänzt"}),
            200,
        )
        .await;

    // Two entries on one date are a single attendance list; re-saving adds nothing.
    let attendance = format!("{prefix}/attendance/2026-10-28");
    let changes = json!({"changes":[{"student_id":s0,"status":"present"},{"student_id":s1,"status":"absent"}]});
    admin.call(&attendance, "PATCH", changes.clone(), 204).await;
    admin.call(&attendance, "PATCH", changes, 204).await;

    let subject = admin
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"Mathematik"}),
            200,
        )
        .await["id"]
        .clone();
    let lesson = admin
        .call(
            "/lessons",
            "POST",
            json!({"classroom_id":class,"subject_id":subject,"lesson_date":"2026-10-29","starts_at":"08:00","ends_at":"08:45"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let performances = format!("/lessons/{lesson}/performances");
    for (student, rating) in [(s0, 3), (s1, 2), (s0, 4), (s0, 4)] {
        admin
            .call(
                &performances,
                "PATCH",
                json!({"student_id":student,"rating":rating}),
                204,
            )
            .await;
    }

    let period = admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            200,
        )
        .await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let criterion = admin.get("/criteria").await["criteria"][0]["id"].clone();
    admin
        .call(
            &format!("{prefix}/periods/{period}/assessments"),
            "PATCH",
            json!({"changes":[{"student_id":s0,"criterion_id":criterion,"value":"yes"},{"student_id":s1,"criterion_id":criterion,"value":"no"}]}),
            204,
        )
        .await;
    // Copying a period's assessments is not new assessment work.
    admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Winter","source_period_id":period}),
            200,
        )
        .await;
    admin
        .raw(
            &format!(
                "{prefix}/periods/{period}/students/{}/pdf",
                s0.as_str().unwrap()
            ),
            "GET",
            Value::Null,
            200,
            true,
        )
        .await;
    admin
        .raw(
            &format!("{prefix}/periods/{period}/zip"),
            "GET",
            Value::Null,
            200,
            true,
        )
        .await;

    let raw = admin
        .raw("/admin/usage", "GET", Value::Null, 200, true)
        .await;
    let text = String::from_utf8(raw).unwrap();
    for private in [
        "usage-admin",
        "usage-teacher",
        "Lehrkraft",
        "Mustermann",
        "Geheime",
        "Vertrauliche",
        &class,
        &lesson,
        s0.as_str().unwrap(),
    ] {
        assert!(!text.contains(private), "usage response contains {private}");
    }
    let usage: Value = serde_json::from_str(&text).unwrap();
    let today: String = sqlx::query_scalar("SELECT date('now')")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(usage["counting_since"], today);
    assert_eq!(usage["from"], today);
    assert_eq!(usage["until"], today);
    assert_eq!(usage["days"], 1);
    assert_eq!(usage["series"].as_array().unwrap().len(), 1);
    assert_eq!(usage["series"][0]["counts"], usage["totals"]);
    assert_eq!(
        usage["totals"],
        json!({
            // Both teachers made requests today; repeated requests count once.
            "teacher_active": 2,
            "session_started": 2,
            "teacher_registered": 2,
            "classroom_created": 1,
            "pupil_added": 2,
            "lesson_created": 1,
            "attendance_taken": 1,
            // Two first ratings, one changed rating; an unchanged re-save is not a correction.
            "performance_recorded": 2,
            "performance_corrected": 1,
            "note_created": 1,
            "note_edited": 1,
            "homework_created": 0,
            "homework_handed_in": 0,
            "homework_rated": 0,
            "worksheet_uploaded": 0,
            "task_created": 0,
            "task_completed": 0,
            "period_created": 2,
            "assessment_recorded": 2,
            "pdf_exported": 1,
            "zip_exported": 1,
        })
    );
    assert_eq!(usage["peak_active_teachers"], 2);

    // Deleting records does not reduce what was counted.
    admin.call(&note_path, "DELETE", Value::Null, 204).await;
    assert_eq!(totals(&mut admin).await["note_created"], 1);
}

#[tokio::test]
async fn usage_sums_days_into_weeks_and_months_from_the_start_of_counting() {
    let (app, db) = fixture().await;
    let mut admin = Client::new(app);
    admin
        .call(
            "/auth/register",
            "POST",
            registration("usage-buckets@example.test", SETUP_TOKEN),
            200,
        )
        .await;
    // Pretend counting started on Wednesday, 2025-01-01.
    sqlx::query("DELETE FROM usage_counters")
        .execute(&db)
        .await
        .unwrap();
    for (day, metric, count) in [
        ("2025-01-01", "counting_started", 1),
        ("2025-01-01", "note_created", 2),
        ("2025-01-05", "note_created", 1),
        ("2025-01-06", "note_created", 4),
        ("2025-01-06", "teacher_active", 3),
        ("2025-01-07", "teacher_active", 1),
        ("2025-02-03", "note_created", 5),
    ] {
        sqlx::query("INSERT INTO usage_counters(day, metric, count) VALUES(?, ?, ?)")
            .bind(day)
            .bind(metric)
            .bind(count)
            .execute(&db)
            .await
            .unwrap();
    }

    // Earlier starts are clamped to the first counted day.
    let weeks = admin
        .get("/admin/usage?from=2020-01-01&until=2025-01-19&bucket=week")
        .await;
    assert_eq!(weeks["from"], "2025-01-01");
    assert_eq!(weeks["days"], 19);
    let starts: Vec<_> = weeks["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["start"].clone(),
                p["days"].clone(),
                p["counts"]["note_created"].clone(),
            )
        })
        .collect();
    assert_eq!(
        starts,
        vec![
            (json!("2024-12-30"), json!(5), json!(3)),
            (json!("2025-01-06"), json!(7), json!(4)),
            (json!("2025-01-13"), json!(7), json!(0)),
        ]
    );
    assert_eq!(weeks["totals"]["note_created"], 7);
    assert_eq!(weeks["totals"]["teacher_active"], 4);
    assert_eq!(weeks["peak_active_teachers"], 3);

    let months = admin
        .get("/admin/usage?until=2025-02-10&bucket=month")
        .await;
    assert_eq!(months["from"], "2025-01-01");
    let starts: Vec<_> = months["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["start"].clone(),
                p["days"].clone(),
                p["counts"]["note_created"].clone(),
            )
        })
        .collect();
    assert_eq!(
        starts,
        vec![
            (json!("2025-01-01"), json!(31), json!(7)),
            (json!("2025-02-01"), json!(10), json!(5)),
        ]
    );

    // A range entirely before counting started is empty rather than an error.
    let before = admin
        .get("/admin/usage?from=2024-01-01&until=2024-12-31")
        .await;
    assert_eq!(before["days"], 0);
    assert_eq!(before["series"], json!([]));
    assert_eq!(before["totals"]["note_created"], 0);

    for query in [
        "bucket=year",
        "from=2025-02-30",
        "until=yesterday",
        "from=2025-02-01&until=2025-01-01",
    ] {
        admin
            .call(&format!("/admin/usage?{query}"), "GET", Value::Null, 400)
            .await;
    }
}

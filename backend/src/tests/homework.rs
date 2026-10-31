use super::*;

async fn setup() -> (Client, Client, SqlitePool, String, Value) {
    let (app, db) = fixture().await;
    let mut owner = Client::new(app.clone());
    let mut editor = Client::new(app);
    owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "homework-owner@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invitation = owner
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Homework editor"}),
            200,
        )
        .await;
    editor
        .call(
            "/auth/register",
            "POST",
            registration(
                "homework-editor@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    let class = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Homework class","school_year":"2026/27"}),
            200,
        )
        .await;
    let prefix = format!("/classrooms/{}", class["id"].as_str().unwrap());
    let subject = owner
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"Mathematik"}),
            200,
        )
        .await;
    (owner, editor, db, prefix, subject)
}

fn input(subject: &Value) -> Value {
    json!({"subject_id":subject["id"],"title":"Zahlen üben","description":"Seite 3\nAufgaben 1–4","due_date":"2026-11-02"})
}

#[tokio::test]
async fn homework_quality_tracks_submissions_and_preserves_closure_rules() {
    let (mut owner, mut editor, _db, prefix, subject) = setup().await;
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Same Name","Same Name"]}),
            200,
        )
        .await;
    let created = owner
        .call(&format!("{prefix}/homework"), "POST", input(&subject), 200)
        .await;
    let hw = format!("{prefix}/homework/{}", created["id"].as_str().unwrap());
    let endpoint = format!("{hw}/submissions");
    let initial = owner.get(&hw).await;
    let p0 = initial["submissions"][0]["student_id"].as_str().unwrap();
    let p1 = initial["submissions"][1]["student_id"].as_str().unwrap();
    assert!(initial["submissions"][0]["quality"].is_null());
    let change = json!({"changes":[{"student_id":p0,"quality":"all_correct"}]});
    editor.call(&endpoint, "PATCH", change.clone(), 404).await;
    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"homework-editor@example.test"}),
            204,
        )
        .await;
    for quality in [
        "all_correct",
        "partially_correct",
        "all_incorrect",
        "missing",
    ] {
        editor
            .call(
                &endpoint,
                "PATCH",
                json!({"changes":[{"student_id":p0,"quality":quality}]}),
                204,
            )
            .await;
        let saved = owner.get(&hw).await;
        assert_eq!(saved["submissions"][0]["quality"], quality);
        assert_eq!(
            saved["submissions"][0]["handed_in_at"].is_null(),
            quality == "missing"
        );
        assert!(saved["submissions"][1]["quality"].is_null());
        assert!(saved["submissions"][1]["handed_in_at"].is_null());
        assert_eq!(saved["homework"]["status"], "open");
    }
    // Legacy checkbox and bulk updates clear an incompatible "missing" quality.
    owner.call(&endpoint, "PATCH", json!({"changes":[{"student_id":p0,"handed_in":true},{"student_id":p1,"quality":"all_incorrect"}]}), 204).await;
    let closed = owner.get(&hw).await;
    assert_eq!(closed["homework"]["status"], "auto_closed");
    assert!(closed["submissions"][0]["quality"].is_null());
    let submitted_at = closed["submissions"][1]["handed_in_at"].clone();
    owner
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":p1,"handed_in":true}]}),
            204,
        )
        .await;
    assert_eq!(owner.get(&hw).await, closed);
    // Correcting and clearing a grade preserve the original hand-in date.
    for quality in [json!("partially_correct"), Value::Null] {
        owner
            .call(
                &endpoint,
                "PATCH",
                json!({"changes":[{"student_id":p1,"quality":quality}]}),
                204,
            )
            .await;
        let saved = owner.get(&hw).await;
        assert_eq!(saved["submissions"][1]["quality"], quality);
        assert_eq!(saved["submissions"][1]["handed_in_at"], submitted_at);
        assert_eq!(saved["homework"]["status"], "auto_closed");
    }
    owner
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":p1,"quality":"missing"}]}),
            204,
        )
        .await;
    let reopened = owner.get(&hw).await;
    assert_eq!(reopened["homework"]["status"], "open");
    assert_eq!(reopened["homework"]["handed_in_count"], 1);
    assert!(reopened["homework"]["closed_at"].is_null());
    for invalid in [
        json!({"student_id":p1,"quality":"unknown"}),
        json!({"student_id":p1,"handed_in":true,"quality":"missing"}),
        json!({"student_id":p1,"handed_in":false,"quality":"all_correct"}),
        json!({"student_id":p1}),
        json!({"student_id":"not-assigned","quality":"all_correct"}),
        json!({"student_id":p0,"quality":"missing"}),
    ] {
        owner
            .call(
                &endpoint,
                "PATCH",
                json!({"changes":[{"student_id":p0,"quality":"all_correct"},invalid]}),
                400,
            )
            .await;
        assert_eq!(owner.get(&hw).await, reopened);
    }
    owner
        .call(
            &format!("{hw}/closure"),
            "PATCH",
            json!({"closed":true}),
            204,
        )
        .await;
    owner.call(&endpoint, "PATCH", change, 204).await;
    owner
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":false}]}),
            204,
        )
        .await;
    let manual = owner.get(&hw).await;
    assert_eq!(manual["homework"]["status"], "manually_closed");
    assert!(manual["submissions"][0]["quality"].is_null());
    assert!(manual["submissions"][0]["handed_in_at"].is_null());
}

#[tokio::test]
async fn homework_quality_migration_preserves_existing_submissions() {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    for migration in [
        include_str!("../../migrations/0001_initial.sql"),
        include_str!("../../migrations/0005_homework.sql"),
    ] {
        sqlx::raw_sql(migration).execute(&db).await.unwrap();
    }
    sqlx::raw_sql("INSERT INTO teachers(id,name,email,password_hash) VALUES('t','Teacher','test@example.test','hash');
      INSERT INTO classrooms(id,name,school_year) VALUES('c','2a','2026/27');
      INSERT INTO students(id,classroom_id,name) VALUES('p','c','Child'),('q','c','Other');
      INSERT INTO subjects(id,classroom_id,name) VALUES('s','c','Deutsch');
      INSERT INTO homework(id,classroom_id,subject_id,title,due_date,created_by) VALUES('h','c','s','Lesen','2026-10-29','t');
      INSERT INTO homework_submissions(homework_id,classroom_id,student_id,handed_in_at) VALUES('h','c','p',123),('h','c','q',NULL);")
      .execute(&db).await.unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0008_homework_quality.sql"))
        .execute(&db)
        .await
        .unwrap();
    let records: Vec<(String, Option<i64>, Option<String>)> = sqlx::query_as(
        "SELECT student_id,handed_in_at,quality FROM homework_submissions ORDER BY student_id",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        records,
        vec![("p".into(), Some(123), None), ("q".into(), None, None)]
    );
    assert!(
        sqlx::query("UPDATE homework_submissions SET quality='invalid'")
            .execute(&db)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn homework_snapshots_rosters_and_closes_atomically_with_shared_access() {
    let (mut owner, mut editor, db, prefix, subject) = setup().await;
    let endpoint = format!("{prefix}/homework");
    let mut anonymous = Client::new(owner.app.clone());
    anonymous.call(&endpoint, "GET", Value::Null, 401).await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    editor
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"Deutsch"}),
            404,
        )
        .await;
    owner
        .raw(&endpoint, "POST", input(&subject), 403, false)
        .await;
    owner.call(&endpoint, "POST", input(&subject), 400).await;
    assert_eq!(owner.get(&endpoint).await, json!([]));
    owner
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"mathematik"}),
            409,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"  "}),
            400,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Same Name","Same Name"]}),
            200,
        )
        .await;
    let workspace = owner.get(&format!("{prefix}/workspace")).await;
    assert_eq!(workspace["periods"], json!([]));
    let pupils = workspace["students"].as_array().unwrap();
    let p0 = pupils[0]["id"].as_str().unwrap();
    let p1 = pupils[1]["id"].as_str().unwrap();
    for date in ["2026-02-29", "0000-01-01", "2026-04-31", "not-a-date"] {
        let mut invalid = input(&subject);
        invalid["due_date"] = json!(date);
        owner.call(&endpoint, "POST", invalid, 400).await;
    }
    let mut invalid = input(&subject);
    invalid["title"] = json!("   ");
    owner.call(&endpoint, "POST", invalid, 400).await;
    let other = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Other","school_year":"2026/27"}),
            200,
        )
        .await;
    let other_prefix = format!("/classrooms/{}", other["id"].as_str().unwrap());
    let foreign_subject = owner
        .call(
            &format!("{other_prefix}/subjects"),
            "POST",
            json!({"name":"Deutsch"}),
            200,
        )
        .await;
    owner
        .call(&endpoint, "POST", input(&foreign_subject), 400)
        .await;
    let created = owner.call(&endpoint, "POST", input(&subject), 200).await;
    let hw = format!("{endpoint}/{}", created["id"].as_str().unwrap());
    let submissions = format!("{hw}/submissions");
    let closure = format!("{hw}/closure");
    let initial = owner.get(&hw).await;
    assert_eq!(initial["homework"]["assigned_count"], 2);
    assert_eq!(initial["homework"]["handed_in_count"], 0);
    assert_eq!(initial["homework"]["status"], "open");
    assert!(initial["submissions"][0]["handed_in_at"].is_null());
    assert_eq!(
        initial["submissions"][0]["name"],
        initial["submissions"][1]["name"]
    );
    assert_ne!(
        initial["submissions"][0]["student_id"],
        initial["submissions"][1]["student_id"]
    );
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Added later"]}),
            200,
        )
        .await;
    let current = owner.get(&format!("{prefix}/workspace")).await;
    let later = current["students"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Added later")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(owner.get(&hw).await["homework"]["assigned_count"], 2);
    owner.call(&submissions, "PATCH", json!({"changes":[{"student_id":p0,"handed_in":true},{"student_id":later,"handed_in":true}]}), 400).await;
    assert_eq!(owner.get(&hw).await, initial);
    owner.call(&submissions, "PATCH", json!({"changes":[{"student_id":p0,"handed_in":true},{"student_id":p0,"handed_in":false}]}), 400).await;
    assert_eq!(owner.get(&hw).await, initial);
    owner
        .call(&submissions, "PATCH", json!({"changes":[]}), 400)
        .await;
    let wrong_class = format!(
        "{other_prefix}/homework/{}",
        created["id"].as_str().unwrap()
    );
    owner.call(&wrong_class, "GET", Value::Null, 404).await;
    owner
        .call(
            &format!("{wrong_class}/submissions"),
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":true}]}),
            404,
        )
        .await;
    editor
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":true}]}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"homework-editor@example.test"}),
            204,
        )
        .await;
    assert_eq!(editor.get(&hw).await, initial);
    editor
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":true}]}),
            204,
        )
        .await;
    let partial = owner.get(&hw).await;
    assert_eq!(partial["homework"]["handed_in_count"], 1);
    assert_eq!(partial["homework"]["status"], "open");
    // Concurrent final checks must see each other's committed writes.
    let a = owner.call(
        &submissions,
        "PATCH",
        json!({"changes":[{"student_id":p0,"handed_in":true}]}),
        204,
    );
    let b = editor.call(
        &submissions,
        "PATCH",
        json!({"changes":[{"student_id":p1,"handed_in":true}]}),
        204,
    );
    tokio::join!(a, b);
    let completed = owner.get(&hw).await;
    assert_eq!(completed["homework"]["status"], "auto_closed");
    assert_eq!(completed["homework"]["handed_in_count"], 2);
    assert!(completed["homework"]["closed_at"].is_i64());
    assert_eq!(
        partial["submissions"][0]["handed_in_at"],
        completed["submissions"][0]["handed_in_at"]
    );
    owner
        .call(&closure, "PATCH", json!({"closed":false}), 400)
        .await;
    owner
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p1,"handed_in":false}]}),
            204,
        )
        .await;
    let reopened = owner.get(&hw).await;
    assert_eq!(reopened["homework"]["status"], "open");
    assert!(reopened["homework"]["closed_at"].is_null());
    owner
        .call(&closure, "PATCH", json!({"closed":true}), 204)
        .await;
    editor
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":false}]}),
            204,
        )
        .await;
    assert_eq!(
        owner.get(&hw).await["homework"]["status"],
        "manually_closed"
    );
    editor
        .call(&closure, "PATCH", json!({"closed":false}), 204)
        .await;
    assert_eq!(owner.get(&hw).await["homework"]["status"], "open");
    let subject_path = format!("{prefix}/subjects/{}", subject["id"].as_str().unwrap());
    editor
        .call(&subject_path, "PATCH", json!({"name":"Rechnen"}), 204)
        .await;
    owner.call(&subject_path, "DELETE", Value::Null, 409).await;
    let mut edited = input(&subject);
    edited["title"] = json!("Korrigierter Titel");
    edited["due_date"] = json!("2026-11-03");
    editor.call(&hw, "PATCH", edited, 204).await;
    editor
        .call(
            &format!("{prefix}/students/{p0}"),
            "PATCH",
            json!({"name":"Renamed pupil"}),
            204,
        )
        .await;
    let renamed = owner.get(&hw).await;
    assert_eq!(renamed["homework"]["subject_name"], "Rechnen");
    assert_eq!(renamed["homework"]["title"], "Korrigierter Titel");
    assert_eq!(renamed["homework"]["due_date"], "2026-11-03");
    assert!(
        renamed["submissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "Renamed pupil")
    );
    owner
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":true}]}),
            204,
        )
        .await;
    editor
        .call(
            &format!("{prefix}/students/{p1}"),
            "DELETE",
            Value::Null,
            403,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/students/{p1}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    let after_removal = owner.get(&hw).await;
    assert_eq!(after_removal["homework"]["assigned_count"], 1);
    assert_eq!(after_removal["homework"]["status"], "auto_closed");
    let editor_id = editor.get("/auth/me").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .call(
            &format!("{prefix}/teachers/{editor_id}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    editor.call(&hw, "GET", Value::Null, 404).await;
    editor
        .call(&closure, "PATCH", json!({"closed":true}), 404)
        .await;
    editor.call(&hw, "PATCH", input(&subject), 404).await;
    editor
        .call(
            &submissions,
            "PATCH",
            json!({"changes":[{"student_id":p0,"handed_in":false}]}),
            404,
        )
        .await;
    let unused = owner
        .call(
            &format!("{prefix}/subjects"),
            "POST",
            json!({"name":"Sport"}),
            200,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/subjects/{}", unused["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    let fk_errors = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&db)
        .await
        .unwrap();
    assert!(fk_errors.is_empty());
}

async fn upload(client: &Client, path: &str, content: Vec<u8>, guard: bool) -> Response {
    let mut builder = Request::builder()
        .method("PUT")
        .uri(format!("/api{path}"))
        .header("cookie", &client.cookie)
        .header("content-type", "application/pdf");
    if guard {
        builder = builder.header("x-tamara-request", "1");
    }
    client
        .app
        .clone()
        .oneshot(builder.body(Body::from(content)).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn homework_pdfs_are_bounded_private_persistent_and_replaceable() {
    let (mut owner, mut editor, db, prefix, subject) = setup().await;
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Pupil"]}),
            200,
        )
        .await;
    let created = owner
        .call(&format!("{prefix}/homework"), "POST", input(&subject), 200)
        .await;
    let hw = format!("{prefix}/homework/{}", created["id"].as_str().unwrap());
    let worksheet = format!("{hw}/worksheet");
    let upload_path = format!("{worksheet}?name=Arbeitsblatt-%C3%BCben.pdf");
    let pdf =
        include_bytes!("../../../entwicklungsuebersicht_mathe_DRUCKVERSION_270925.pdf").to_vec();
    owner.call(&worksheet, "GET", Value::Null, 404).await;
    assert_eq!(
        upload(&editor, &upload_path, pdf.clone(), true)
            .await
            .status(),
        404
    );
    assert_eq!(
        upload(&owner, &upload_path, pdf.clone(), false)
            .await
            .status(),
        403
    );
    assert_eq!(
        upload(&owner, &upload_path, b"This is not a PDF".to_vec(), true)
            .await
            .status(),
        400
    );
    assert_eq!(
        upload(
            &owner,
            &format!("{worksheet}?name=bad.txt"),
            pdf.clone(),
            true
        )
        .await
        .status(),
        400
    );
    assert_eq!(
        upload(
            &owner,
            &format!("{worksheet}?name=..%2Ffile.pdf"),
            pdf.clone(),
            true
        )
        .await
        .status(),
        400
    );
    let too_large = vec![b'x'; 10 * 1024 * 1024 + 1];
    assert_eq!(
        upload(&owner, &upload_path, too_large, true).await.status(),
        413
    );
    // This exceeds the normal API's 1 MB limit but is valid for worksheet routes.
    let mut large_pdf = b"%PDF-1.4\n%".to_vec();
    large_pdf.resize(2 * 1024 * 1024, b' ');
    large_pdf.extend_from_slice(b"\n%%EOF\n");
    assert_eq!(
        upload(&owner, &upload_path, large_pdf, true).await.status(),
        204
    );
    assert_eq!(
        owner.get(&hw).await["homework"]["worksheet_name"],
        "Arbeitsblatt-üben.pdf"
    );
    assert_eq!(
        upload(&owner, &upload_path, pdf.clone(), true)
            .await
            .status(),
        204
    );
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT content FROM homework_worksheets WHERE homework_id=?")
            .bind(created["id"].as_str().unwrap())
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(stored, pdf);
    let response = owner
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api{worksheet}"))
                .header("cookie", &owner.cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        response.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .contains("filename*=UTF-8''")
    );
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        pdf
    );
    owner
        .call(
            &format!("{hw}/closure"),
            "PATCH",
            json!({"closed":true}),
            204,
        )
        .await;
    assert_eq!(
        owner.raw(&worksheet, "GET", Value::Null, 200, true).await,
        pdf
    );
    editor.call(&worksheet, "GET", Value::Null, 404).await;
    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"homework-editor@example.test"}),
            204,
        )
        .await;
    assert_eq!(
        editor.raw(&worksheet, "GET", Value::Null, 200, true).await,
        pdf
    );
    let editor_id = editor.get("/auth/me").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .call(
            &format!("{prefix}/teachers/{editor_id}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    editor.call(&worksheet, "GET", Value::Null, 404).await;
    assert_eq!(upload(&editor, &upload_path, pdf, true).await.status(), 404);
    editor.call(&worksheet, "DELETE", Value::Null, 404).await;
    owner.call(&worksheet, "DELETE", Value::Null, 204).await;
    assert!(owner.get(&hw).await["homework"]["worksheet_name"].is_null());
    owner.call(&worksheet, "GET", Value::Null, 404).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM homework_worksheets")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

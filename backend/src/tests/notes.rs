use super::*;

#[tokio::test]
async fn notes_are_dated_shared_isolated_and_follow_the_child() {
    let (app, db) = fixture().await;
    let mut owner = Client::new(app.clone());
    owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "notes-owner@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invitation = owner
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Notes editor"}),
            200,
        )
        .await;
    let mut editor = Client::new(app.clone());
    let teacher = editor
        .call(
            "/auth/register",
            "POST",
            registration(
                "notes-editor@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    let classroom = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Notes class","school_year":"2026/27"}),
            200,
        )
        .await;
    let prefix = format!("/classrooms/{}", classroom["id"].as_str().unwrap());
    let endpoint = format!("{prefix}/notes");
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Same Name","Same Name"]}),
            200,
        )
        .await;
    let workspace = owner.get(&format!("{prefix}/workspace")).await;
    let pupil = &workspace["students"][0]["id"];
    let other_pupil = &workspace["students"][1]["id"];
    assert_eq!(workspace["periods"], json!([]));
    let input = json!({"student_id":pupil,"content":"  Beobachtung: größer als zuvor.\nNächster Schritt: üben.  "});

    let mut anonymous = Client::new(app);
    anonymous.call(&endpoint, "GET", Value::Null, 401).await;
    anonymous.call(&endpoint, "POST", input.clone(), 401).await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    editor.call(&endpoint, "POST", input.clone(), 404).await;
    owner
        .raw(&endpoint, "POST", input.clone(), 403, false)
        .await;
    for invalid in [
        "".to_string(),
        " \n\t ".into(),
        "x".repeat(10001),
        "bad\u{0000}text".into(),
    ] {
        owner
            .call(
                &endpoint,
                "POST",
                json!({"student_id":pupil,"content":invalid}),
                400,
            )
            .await;
    }
    owner
        .call(
            &endpoint,
            "POST",
            json!({"student_id":"unknown","content":"Note"}),
            404,
        )
        .await;
    assert_eq!(owner.get(&endpoint).await, json!([]));

    let before = now();
    let first = owner.call(&endpoint, "POST", input, 200).await;
    assert_eq!(first["student_id"], *pupil);
    assert_eq!(
        first["content"],
        "Beobachtung: größer als zuvor.\nNächster Schritt: üben."
    );
    assert!(first["created_at"].as_u64().unwrap() >= before);
    assert!(first["created_at"].as_u64().unwrap() <= now());
    assert_eq!(first["updated_at"], first["created_at"]);
    let note_path = format!("{endpoint}/{}", first["id"].as_str().unwrap());
    let second = owner
        .call(
            &endpoint,
            "POST",
            json!({"student_id":other_pupil,"content":"<b>Plain text</b>"}),
            200,
        )
        .await;
    let third = owner
        .call(
            &endpoint,
            "POST",
            json!({"student_id":pupil,"content":"Weitere Notiz"}),
            200,
        )
        .await;
    let list = owner.get(&endpoint).await;
    assert_eq!(list, json!([third, second, first]));

    let other_class = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Other class","school_year":"2026/27"}),
            200,
        )
        .await;
    let foreign_endpoint = format!("/classrooms/{}/notes", other_class["id"].as_str().unwrap());
    owner
        .call(
            &foreign_endpoint,
            "POST",
            json!({"student_id":pupil,"content":"Wrong class"}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{foreign_endpoint}/{}", first["id"].as_str().unwrap()),
            "PATCH",
            json!({"content":"Wrong class"}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{foreign_endpoint}/{}", first["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            404,
        )
        .await;
    assert_eq!(owner.get(&foreign_endpoint).await, json!([]));

    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"notes-editor@example.test"}),
            204,
        )
        .await;
    assert_eq!(editor.get(&endpoint).await, list);
    let updated = editor
        .call(
            &note_path,
            "PATCH",
            json!({"content":"Korrigiert\nmit zweiter Zeile"}),
            200,
        )
        .await;
    assert_eq!(updated["created_at"], first["created_at"]);
    assert_eq!(updated["created_by"], first["created_by"]);
    assert_eq!(updated["updated_by"], teacher["id"]);
    editor
        .call(&note_path, "PATCH", json!({"content":"  "}), 400)
        .await;
    assert_eq!(owner.get(&endpoint).await[2], updated);

    let pupil_path = format!("{prefix}/students/{}", pupil.as_str().unwrap());
    editor
        .call(&pupil_path, "PATCH", json!({"name":"Renamed Child"}), 204)
        .await;
    assert_eq!(editor.get(&endpoint).await[2]["student_id"], *pupil);
    owner
        .call(
            &format!("{prefix}/teachers/{}", teacher["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    editor
        .call(
            &endpoint,
            "POST",
            json!({"student_id":pupil,"content":"Revoked"}),
            404,
        )
        .await;
    editor
        .call(&note_path, "PATCH", json!({"content":"Revoked"}), 404)
        .await;
    editor.call(&note_path, "DELETE", Value::Null, 404).await;

    owner.call(&note_path, "DELETE", Value::Null, 204).await;
    owner.call(&note_path, "DELETE", Value::Null, 404).await;
    owner.call(&pupil_path, "DELETE", Value::Null, 204).await;
    assert_eq!(owner.get(&endpoint).await, json!([second]));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM student_notes")
            .fetch_one(&db)
            .await
            .unwrap(),
        1
    );
}

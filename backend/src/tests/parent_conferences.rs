use super::*;

async fn document_request(
    client: &Client,
    path: &str,
    method: &str,
    bytes: Vec<u8>,
    status: u16,
) -> Response {
    let response = client
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/api{path}"))
                .header("cookie", &client.cookie)
                .header("x-tamara-request", "1")
                .header("content-type", "application/octet-stream")
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), status);
    response
}

#[tokio::test]
async fn parent_conferences_preserve_reviews_documents_and_access_boundaries() {
    let (app, db) = fixture().await;
    let mut owner = Client::new(app.clone());
    owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "parent-owner@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invite = owner
        .call("/admin/invitations", "POST", json!({"label":"editor"}), 200)
        .await;
    let mut editor = Client::new(app.clone());
    let teacher = editor
        .call(
            "/auth/register",
            "POST",
            registration(
                "parent-editor@example.test",
                invite["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    let class = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"1a","school_year":"2026/27"}),
            200,
        )
        .await;
    let prefix = format!("/classrooms/{}", class["id"].as_str().unwrap());
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Same Name","Same Name"]}),
            200,
        )
        .await;
    let workspace = owner.get(&format!("{prefix}/workspace")).await;
    let pupil = workspace["students"][0]["id"].clone();
    let other_pupil = workspace["students"][1]["id"].clone();
    assert_eq!(
        owner
            .get("/parent-conferences/pupils")
            .await
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(editor.get("/parent-conferences/pupils").await, json!([]));
    let input = json!({"student_id":pupil,"date":"2026-11-03","content":"First\nSecond","attendees":[" Mother ","Father"]});
    owner
        .call("/parent-conferences/meeting", "PUT", input.clone(), 204)
        .await;
    assert_eq!(
        owner.get("/parent-conferences").await[0]["attendees"],
        json!(["Mother", "Father"])
    );
    let mut invalid = input.clone();
    invalid["date"] = json!("2026-02-30");
    owner
        .call("/parent-conferences/meeting", "PUT", invalid, 400)
        .await;
    let mut reassigned = input.clone();
    reassigned["student_id"] = other_pupil;
    owner
        .call("/parent-conferences/meeting", "PUT", reassigned, 404)
        .await;
    let mut invalid = input.clone();
    invalid["attendees"] = json!([""]);
    owner
        .call("/parent-conferences/meeting", "PUT", invalid, 400)
        .await;
    let mut anonymous = Client::new(app.clone());
    anonymous
        .call("/parent-conferences", "GET", Value::Null, 401)
        .await;
    editor
        .call("/parent-conferences/meeting", "PUT", input.clone(), 404)
        .await;
    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"parent-editor@example.test"}),
            204,
        )
        .await;
    editor
        .call("/parent-conferences/meeting", "PUT", input.clone(), 404)
        .await;
    assert_eq!(editor.get("/parent-conferences").await, json!([]));
    editor
        .call(
            "/parent-conferences/editor-meeting",
            "PUT",
            input.clone(),
            204,
        )
        .await;
    let period = owner
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            200,
        )
        .await;
    let catalog = owner.get("/criteria").await;
    let yes = catalog["criteria"][0]["id"].clone();
    let no = catalog["criteria"][1]["id"].clone();
    let assessments = format!(
        "{prefix}/periods/{}/assessments",
        period["id"].as_str().unwrap()
    );
    owner.call(&assessments,"PATCH",json!({"changes":[{"student_id":pupil,"criterion_id":yes,"value":"yes","achieved_on":"2026-11-01"},{"student_id":pupil,"criterion_id":no,"value":"no"}]}),204).await;
    owner
        .call(
            "/parent-conferences/meeting/review",
            "POST",
            json!({"period_id":"missing"}),
            404,
        )
        .await;
    owner
        .call(
            "/parent-conferences/meeting/review",
            "POST",
            json!({"period_id":period["id"]}),
            204,
        )
        .await;
    let snapshot = owner.get("/parent-conferences").await[0]["review_snapshot"].clone();
    let marks = snapshot["assessments"].as_array().unwrap();
    assert_eq!(marks.len(), 354);
    assert_eq!(marks.iter().filter(|a| a["value"] == "yes").count(), 1);
    assert_eq!(marks.iter().filter(|a| a["value"] == "no").count(), 1);
    assert_eq!(marks.iter().filter(|a| a["value"].is_null()).count(), 352);
    assert_eq!(
        marks.iter().find(|a| a["criterion_id"] == yes).unwrap()["achieved_on"],
        "2026-11-01"
    );
    owner
        .call(
            &assessments,
            "PATCH",
            json!({"changes":[{"student_id":pupil,"criterion_id":yes,"value":"no"}]}),
            204,
        )
        .await;
    owner
        .call("/parent-conferences/meeting", "PUT", input.clone(), 204)
        .await;
    owner
        .call(
            "/parent-conferences/meeting/review",
            "POST",
            json!({"period_id":period["id"]}),
            400,
        )
        .await;
    assert_eq!(
        owner.get("/parent-conferences").await[0]["review_snapshot"],
        snapshot
    );
    let bytes = b"a private document".to_vec();
    document_request(
        &owner,
        "/parent-conferences/meeting/documents?name=Notiz%20%C3%A4.txt",
        "POST",
        bytes.clone(),
        204,
    )
    .await;
    document_request(
        &owner,
        "/parent-conferences/meeting/documents?name=..%2Fbad",
        "POST",
        bytes.clone(),
        400,
    )
    .await;
    document_request(
        &owner,
        "/parent-conferences/meeting/documents?name=empty",
        "POST",
        vec![],
        400,
    )
    .await;
    document_request(
        &owner,
        "/parent-conferences/meeting/documents?name=big",
        "POST",
        vec![0; 10 * 1024 * 1024 + 1],
        413,
    )
    .await;
    let docs = owner.get("/parent-conferences/meeting/documents").await;
    let path = format!(
        "/parent-conferences/meeting/documents/{}",
        docs[0]["id"].as_str().unwrap()
    );
    let response = document_request(&owner, &path, "GET", vec![], 200).await;
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
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
        bytes
    );
    editor.call(&path, "GET", Value::Null, 404).await;
    editor.call(&path, "DELETE", Value::Null, 404).await;
    owner
        .call("/parent-conferences/other", "PUT", input.clone(), 204)
        .await;
    let wrong = format!(
        "/parent-conferences/other/documents/{}",
        docs[0]["id"].as_str().unwrap()
    );
    owner.call(&wrong, "GET", Value::Null, 404).await;
    owner
        .call(
            &format!("{prefix}/teachers/{}", teacher["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    assert_eq!(editor.get("/parent-conferences").await, json!([]));
    editor
        .call(
            "/parent-conferences/editor-meeting/documents",
            "GET",
            Value::Null,
            404,
        )
        .await;
    editor
        .call("/parent-conferences/editor-meeting", "PUT", input, 404)
        .await;
    owner
        .call(
            "/parent-conferences/meeting/review",
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    assert!(
        owner
            .get("/parent-conferences")
            .await
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "meeting")
            .unwrap()["review_snapshot"]
            .is_null()
    );
    owner.call(&path, "DELETE", Value::Null, 204).await;
    assert_eq!(
        owner.get("/parent-conferences/meeting/documents").await,
        json!([])
    );
    document_request(
        &owner,
        "/parent-conferences/meeting/documents?name=again",
        "POST",
        bytes,
        204,
    )
    .await;
    owner
        .call("/parent-conferences/meeting", "DELETE", Value::Null, 204)
        .await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM parent_conference_documents")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    owner
        .call(
            &format!("{prefix}/students/{}", pupil.as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    assert_eq!(owner.get("/parent-conferences").await, json!([]));
}

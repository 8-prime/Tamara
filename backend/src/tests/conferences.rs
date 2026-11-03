use super::*;
#[tokio::test]
async fn conferences_roundtrip_validate_and_isolate_accounts() {
    let (app, _) = fixture().await;
    let mut owner = Client::new(app.clone());
    owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "conference@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let input =
        json!({"date":"2026-11-03","title":null,"classroom_id":null,"content":"First\nSecond"});
    owner
        .call("/conferences/test", "PUT", input.clone(), 200)
        .await;
    assert_eq!(
        owner.get("/conferences").await[0]["content"],
        "First\nSecond"
    );
    let mut invalid = input.clone();
    invalid["date"] = json!("2026-02-30");
    owner.call("/conferences/test", "PUT", invalid, 400).await;
    let invitation = owner
        .call("/admin/invitations", "POST", json!({"label":"Other"}), 200)
        .await;
    let mut other = Client::new(app.clone());
    other
        .call(
            "/auth/register",
            "POST",
            registration(
                "conference-other@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    assert_eq!(other.get("/conferences").await, json!([]));
    other
        .call("/conferences/test", "PUT", input.clone(), 404)
        .await;
    other
        .call("/conferences/test", "DELETE", Value::Null, 404)
        .await;
    let classroom = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Conference class","school_year":"2026/27"}),
            200,
        )
        .await;
    let mut linked = input.clone();
    linked["classroom_id"] = classroom["id"].clone();
    assert_eq!(
        owner
            .call("/conferences/test", "PUT", linked.clone(), 200)
            .await["classroom_id"],
        classroom["id"]
    );
    other.call("/conferences/other", "PUT", linked, 404).await;
    let mut anonymous = Client::new(app);
    anonymous
        .call("/conferences", "GET", Value::Null, 401)
        .await;
    let mut edit = input;
    edit["title"] = json!("Title");
    edit["content"] = json!("");
    assert_eq!(
        owner.call("/conferences/test", "PUT", edit, 200).await["title"],
        "Title"
    );
    owner
        .call("/conferences/test", "DELETE", Value::Null, 204)
        .await;
    assert_eq!(owner.get("/conferences").await, json!([]));
}

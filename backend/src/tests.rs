use super::*;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

struct Client {
    app: Router,
    cookie: String,
}
impl Client {
    fn new(app: Router) -> Self {
        Self {
            app,
            cookie: String::new(),
        }
    }
    async fn raw(
        &mut self,
        path: &str,
        method: &str,
        data: Value,
        status: u16,
        guard: bool,
    ) -> Vec<u8> {
        let mut req = Request::builder()
            .method(method)
            .uri(format!("/api{path}"))
            .header("content-type", "application/json");
        if guard {
            req = req.header("x-tamara-request", "1");
        }
        if !self.cookie.is_empty() {
            req = req.header("cookie", &self.cookie);
        }
        let req = req
            .body(if data.is_null() {
                Body::empty()
            } else {
                Body::from(data.to_string())
            })
            .unwrap();
        let response = self.app.clone().oneshot(req).await.unwrap();
        if let Some(cookie) = response.headers().get(header::SET_COOKIE) {
            self.cookie = cookie.to_str().unwrap().split(';').next().unwrap().into();
        }
        let actual = response.status().as_u16();
        let bytes = response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert_eq!(
            actual,
            status,
            "{method} {path}: {}",
            String::from_utf8_lossy(&bytes)
        );
        bytes
    }
    async fn call(&mut self, path: &str, method: &str, data: Value, status: u16) -> Value {
        let bytes = self.raw(path, method, data, status, true).await;
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        }
    }
    async fn get(&mut self, path: &str) -> Value {
        self.call(path, "GET", Value::Null, 200).await
    }
}
async fn fixture() -> (Router, SqlitePool) {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&db).await.unwrap();
    for c in &pdf::catalog().criteria {
        sqlx::query("INSERT INTO math_criteria(id,label,domain,section,level,page,x,y) VALUES(?,?,?,?,?,?,?,?)").bind(&c.id).bind(&c.label).bind(&c.domain).bind(&c.section).bind(&c.level).bind(c.page).bind(c.x).bind(c.y).execute(&db).await.unwrap();
    }
    let state = AppState {
        db: db.clone(),
        secure_cookie: true,
        setup_hash: Some(hash("fixture-setup-token-at-least-24-characters")),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    (routes(state), db)
}
fn registration(email: &str, token: &str) -> Value {
    json!({"name":"Lehrkraft Test","email":email,"password":"long-test-password-123","registration_token":token})
}

#[tokio::test]
async fn complete_teaching_workflow_and_permissions() {
    let (app, db) = fixture().await;
    let mut admin = Client::new(app.clone());
    let mut teacher = Client::new(app.clone());
    let mut stranger = Client::new(app.clone());
    admin.call("/classrooms", "GET", Value::Null, 401).await;
    assert_eq!(admin.get("/auth/status").await["setup_required"], true);
    admin
        .call(
            "/auth/register",
            "POST",
            registration("missing@example.test", ""),
            400,
        )
        .await;
    admin
        .raw(
            "/auth/register",
            "POST",
            registration(
                "blocked@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            403,
            false,
        )
        .await;
    let first = admin
        .call(
            "/auth/register",
            "POST",
            registration(
                "admin@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    assert_eq!(first["is_admin"], true);
    assert_eq!(admin.get("/auth/status").await["setup_required"], false);
    let invite = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Neue Lehrkraft"}),
            200,
        )
        .await;
    let list = admin.get("/admin/invitations").await;
    assert!(list[0].get("token").is_none() && list[0].get("token_hash").is_none());
    let second = teacher
        .call(
            "/auth/register",
            "POST",
            registration("teacher@example.test", invite["token"].as_str().unwrap()),
            200,
        )
        .await;
    assert_eq!(second["is_admin"], false);
    stranger
        .call(
            "/auth/register",
            "POST",
            registration("reused@example.test", invite["token"].as_str().unwrap()),
            400,
        )
        .await;
    stranger
        .call(
            "/auth/register",
            "POST",
            registration(
                "setup-reused@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            400,
        )
        .await;
    teacher
        .call("/admin/invitations", "GET", Value::Null, 403)
        .await;
    let revoked = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Widerrufen"}),
            200,
        )
        .await;
    admin
        .call(
            &format!("/admin/invitations/{}", revoked["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    stranger
        .call(
            "/auth/register",
            "POST",
            registration("revoked@example.test", revoked["token"].as_str().unwrap()),
            400,
        )
        .await;
    let expired = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Abgelaufen"}),
            200,
        )
        .await;
    sqlx::query("UPDATE registration_tokens SET expires_at=unixepoch()-1 WHERE id=?")
        .bind(expired["id"].as_str().unwrap())
        .execute(&db)
        .await
        .unwrap();
    stranger
        .call(
            "/auth/register",
            "POST",
            registration("expired@example.test", expired["token"].as_str().unwrap()),
            400,
        )
        .await;
    let cls = admin
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Klasse 2a","school_year":"2026/27"}),
            200,
        )
        .await;
    let class = cls["id"].as_str().unwrap();
    let prefix = format!("/classrooms/{class}");
    assert_eq!(teacher.get("/classrooms").await, json!([]));
    teacher
        .call(&format!("{prefix}/workspace"), "GET", Value::Null, 404)
        .await;
    admin
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Amira Yılmaz","Mia Müller","Jonas Schmidt"]}),
            200,
        )
        .await;
    let p = admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            200,
        )
        .await;
    let period = p["id"].as_str().unwrap();
    let w = admin.get(&format!("{prefix}/workspace")).await;
    let students = w["students"].as_array().unwrap();
    assert_eq!(students.len(), 3);
    assert_eq!(w["assessments"], json!([]));
    let s0 = students[0]["id"].as_str().unwrap();
    let s1 = students[1]["id"].as_str().unwrap();
    let s2 = students[2]["id"].as_str().unwrap();
    let catalog = admin.get("/criteria").await;
    let criteria = catalog["criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 354);
    let c0 = criteria[0]["id"].as_str().unwrap();
    let last = criteria.last().unwrap()["id"].as_str().unwrap();
    let endpoint = format!("{prefix}/periods/{period}/assessments");
    admin.call(&endpoint,"PATCH",json!({"changes":[{"student_id":s0,"criterion_id":c0,"value":"yes"},{"student_id":s1,"criterion_id":c0,"value":"no"}]}),204).await;
    // Invalid batches change nothing, including valid rows preceding the error.
    admin.call(&endpoint,"PATCH",json!({"changes":[{"student_id":s0,"criterion_id":c0,"value":"no"},{"student_id":"invalid","criterion_id":c0,"value":"yes"}]}),400).await;
    let w = admin.get(&format!("{prefix}/workspace")).await;
    assert_eq!(
        w["assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["student_id"] == s0)
            .unwrap()["value"],
        "yes"
    );
    let copied = admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Winter","source_period_id":period}),
            200,
        )
        .await;
    let copy = copied["id"].as_str().unwrap();
    let w = admin
        .get(&format!("{prefix}/workspace?period_id={copy}"))
        .await;
    assert_eq!(w["assessments"].as_array().unwrap().len(), 2);
    admin
        .call(
            &format!("{prefix}/periods/{copy}/assessments"),
            "PATCH",
            json!({"changes":[{"student_id":s0,"criterion_id":c0,"value":null}]}),
            204,
        )
        .await;
    assert_eq!(
        admin
            .get(&format!("{prefix}/workspace?period_id={copy}"))
            .await["assessments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        admin
            .get(&format!("{prefix}/workspace?period_id={period}"))
            .await["assessments"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Duplicate names do not silently overwrite an existing grading period.
    admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            409,
        )
        .await;
    admin
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"teacher@example.test"}),
            204,
        )
        .await;
    assert_eq!(
        teacher.get("/classrooms").await.as_array().unwrap().len(),
        1
    );
    teacher
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"admin@example.test"}),
            403,
        )
        .await;
    teacher.call(&endpoint,"PATCH",json!({"changes":students.iter().map(|s|json!({"student_id":s["id"],"criterion_id":last,"value":"yes"})).collect::<Vec<_>>()}),204).await;
    assert_eq!(
        admin
            .get(&format!("{prefix}/workspace?period_id={period}"))
            .await["assessments"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    // Cross-class references cannot be used for reads, copying, or writes.
    let cls2 = admin
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Klasse 3b","school_year":"2026/27"}),
            200,
        )
        .await;
    let other = cls2["id"].as_str().unwrap();
    admin
        .call(
            &format!("/classrooms/{other}/periods"),
            "POST",
            json!({"name":"Invalid","source_period_id":period}),
            404,
        )
        .await;
    admin
        .call(
            &format!("/classrooms/{other}/periods/{period}/assessments"),
            "PATCH",
            json!({"changes":[{"student_id":s0,"criterion_id":c0,"value":"yes"}]}),
            404,
        )
        .await;
    // Write one fully marked export for visual QA across every grading page.
    admin.call(&endpoint,"PATCH",json!({"changes":criteria.iter().map(|c|json!({"student_id":s0,"criterion_id":c["id"],"value":"yes"})).collect::<Vec<_>>()}),204).await;
    let pdfpath = format!("{prefix}/periods/{period}/students/{s0}/pdf");
    stranger.call(&pdfpath, "GET", Value::Null, 401).await;
    let bytes = admin.raw(&pdfpath, "GET", Value::Null, 200, true).await;
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        lopdf::Document::load_mem(&bytes).unwrap().get_pages().len(),
        20
    );
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../output/pdf");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("qa-all-criteria.pdf"), bytes).unwrap();
    admin
        .call(
            &format!("{prefix}/teachers/{}", second["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    teacher
        .call(&format!("{prefix}/workspace"), "GET", Value::Null, 404)
        .await;
    teacher.call(&pdfpath, "GET", Value::Null, 404).await;
    admin
        .call(
            &format!("{prefix}/students/{s0}"),
            "PATCH",
            json!({"name":"Amira Öztürk"}),
            204,
        )
        .await;
    teacher
        .call(
            &format!("{prefix}/students/{s0}"),
            "PATCH",
            json!({"name":"Unauthorized"}),
            404,
        )
        .await;
    admin
        .call(
            &format!("{prefix}/students/{s2}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    assert_eq!(
        admin.get(&format!("{prefix}/workspace")).await["students"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM math_assessments WHERE student_id=?")
            .bind(s2)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    teacher.call("/auth/logout", "POST", Value::Null, 204).await;
    teacher.call("/auth/me", "GET", Value::Null, 401).await;
    teacher
        .call(
            "/auth/login",
            "POST",
            json!({"email":"teacher@example.test","password":"wrong"}),
            401,
        )
        .await;
    teacher
        .call(
            "/auth/login",
            "POST",
            json!({"email":"teacher@example.test","password":"long-test-password-123"}),
            200,
        )
        .await;
    assert_eq!(teacher.get("/auth/me").await["id"], second["id"]);
}

#[tokio::test]
async fn invitation_can_only_be_claimed_once_concurrently() {
    let (app, db) = fixture().await;
    let mut admin = Client::new(app.clone());
    admin
        .call(
            "/auth/register",
            "POST",
            registration(
                "admin-race@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let inv = admin
        .call("/admin/invitations", "POST", json!({"label":"Race"}), 200)
        .await;
    let token = inv["token"].as_str().unwrap();
    let requests = [
        registration("race1@example.test", token),
        registration("race2@example.test", token),
    ];
    let mut tasks = vec![];
    for data in requests {
        let a = app.clone();
        tasks.push(tokio::spawn(async move {
            a.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("content-type", "application/json")
                    .header("x-tamara-request", "1")
                    .body(Body::from(data.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
            .as_u16()
        }));
    }
    let mut statuses = vec![];
    for t in tasks {
        statuses.push(t.await.unwrap());
    }
    statuses.sort();
    assert_eq!(statuses, vec![200, 400]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teachers")
            .fetch_one(&db)
            .await
            .unwrap(),
        2
    );
}

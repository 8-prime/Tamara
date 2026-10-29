use super::*;
use axum::http::Request;
use http_body_util::BodyExt;
use tower::ServiceExt;

mod homework;
mod notes;

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
    fixture_database(db).await
}
async fn fixture_database(db: SqlitePool) -> (Router, SqlitePool) {
    sqlx::migrate!("./migrations").run(&db).await.unwrap();
    for c in &pdf::catalog().criteria {
        sqlx::query("INSERT INTO math_criteria(id,label,domain,section,level,page,x,y) VALUES(?,?,?,?,?,?,?,?)").bind(&c.id).bind(&c.label).bind(&c.domain).bind(&c.section).bind(&c.level).bind(c.page).bind(c.x).bind(c.y).execute(&db).await.unwrap();
    }
    let state = AppState {
        db: db.clone(),
        backups: None,
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
async fn only_admins_can_create_backups_and_archives_are_not_exposed() {
    let root = std::env::temp_dir().join(format!("tamara-backup-api-{}", id()));
    let dist = root.join("frontend");
    std::fs::create_dir_all(&dist).unwrap();
    let db = SqlitePoolOptions::new()
        .connect_with(
            SqliteConnectOptions::new()
                .filename(root.join("database.sqlite"))
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal),
        )
        .await
        .unwrap();
    let (_, db) = fixture_database(db).await;
    let html = "<!doctype html><title>Tamara</title>";
    std::fs::write(dist.join("index.html"), html).unwrap();
    let app = routes_with_frontend(
        AppState {
            db: db.clone(),
            backups: Some(Arc::new(backup::Backups::new(
                db.clone(),
                &root.join("database.sqlite"),
            ))),
            secure_cookie: true,
            setup_hash: Some(hash("fixture-setup-token-at-least-24-characters")),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        },
        &dist,
    );
    let mut admin = Client::new(app.clone());
    admin.call("/admin/backups", "POST", Value::Null, 401).await;
    admin
        .call(
            "/auth/register",
            "POST",
            registration(
                "backup-admin@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invitation = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Teacher"}),
            200,
        )
        .await;
    let mut teacher = Client::new(app.clone());
    teacher
        .call(
            "/auth/register",
            "POST",
            registration(
                "backup-teacher@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    teacher
        .call("/admin/backups", "POST", Value::Null, 403)
        .await;
    admin
        .raw("/admin/backups", "POST", Value::Null, 403, false)
        .await;
    let directory = root.join("database.sqlite.backups");
    assert!(!directory.exists());
    admin.call("/admin/backups", "POST", Value::Null, 204).await;
    let archives: Vec<_> = std::fs::read_dir(directory.join("manual"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(archives.len(), 1);
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&archives[0]).unwrap()).unwrap();
    assert_eq!(archive.len(), 1);
    assert!(archive.by_name("database.sqlite").unwrap().size() > 0);
    admin
        .raw("/admin/backups", "GET", Value::Null, 405, true)
        .await;
    admin
        .call("/admin/backups/anything.zip", "GET", Value::Null, 404)
        .await;
    for path in [
        "/database.sqlite.backups/".to_string(),
        format!(
            "/database.sqlite.backups/manual/{}",
            archives[0].file_name().unwrap().to_str().unwrap()
        ),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.headers()[header::CONTENT_TYPE], "text/html");
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            html
        );
    }
    drop(archive);
    db.close().await;
    std::fs::remove_dir_all(root).unwrap();
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
    teacher
        .call(
            &format!("{prefix}/periods/{period}/zip"),
            "GET",
            Value::Null,
            404,
        )
        .await;
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

#[tokio::test]
async fn serves_frontend_and_api_from_the_same_router() {
    let (_, db) = fixture().await;
    let dist = std::env::temp_dir().join(format!("tamara-frontend-test-{}", id()));
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    let html = "<!doctype html><html><title>Tamara</title><body>Frontend</body></html>";
    std::fs::write(dist.join("index.html"), html).unwrap();
    std::fs::write(dist.join("assets/app.js"), "console.log('tamara');").unwrap();
    std::fs::write(dist.join("assets/app.css"), "body{color:green}").unwrap();
    let app = routes_with_frontend(
        AppState {
            db,
            backups: None,
            secure_cookie: false,
            setup_hash: Some(hash("fixture-setup-token-at-least-24-characters")),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        },
        &dist,
    );
    for path in ["/", "/classrooms/example"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            html
        );
    }
    for (path, mime) in [
        ("/assets/app.js", "javascript"),
        ("/assets/app.css", "text/css"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .contains(mime)
        );
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    for path in ["/api", "/api/does-not-exist"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/missing.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_ne!(
        response.into_body().collect().await.unwrap().to_bytes(),
        html
    );
    let mut client = Client::new(app);
    assert_eq!(client.get("/auth/status").await["setup_required"], true);
    std::fs::remove_dir_all(dist).unwrap();
}

// Check archive contents against PDF drawing operations, not just ZIP signatures.
fn assert_period_archive(
    bytes: Vec<u8>,
    students: &[Value],
    marks: &HashMap<String, usize>,
    period: &str,
) {
    use std::io::Read;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(archive.len(), students.len());
    let template = lopdf::Document::load_mem(include_bytes!(
        "../../entwicklungsuebersicht_mathe_DRUCKVERSION_270925.pdf"
    ))
    .unwrap();
    let source_pages = template.get_pages();
    let mut names = HashSet::new();
    for (index, student) in students.iter().enumerate() {
        let mut entry = archive.by_index(index).unwrap();
        let filename = entry.name().to_owned();
        assert!(names.insert(filename.clone()));
        assert!(filename.starts_with(&format!("{:03}-Mathematik-", index + 1)));
        assert!(filename.ends_with(&format!("-{period}.pdf")));
        assert!(!filename.contains('/') && !filename.contains('\\'));
        assert!(entry.enclosed_name().is_some());
        let mut pdf = Vec::new();
        entry.read_to_end(&mut pdf).unwrap();
        let doc = lopdf::Document::load_mem(&pdf).unwrap();
        assert_eq!(doc.get_pages().len(), 20);
        let cover = doc.extract_text(&[1]).unwrap();
        assert!(cover.contains(student["name"].as_str().unwrap()), "{cover}");
        assert!(cover.contains(period) && cover.contains("2026/27"));
        let mut crosses = 0;
        for (page, object) in doc.get_pages() {
            if page == 1 {
                continue;
            }
            let streams = doc.get_page_contents(object);
            let original = template.get_page_contents(source_pages[&page]).len();
            assert!(streams.len() == original || streams.len() == original + 1);
            if streams.len() > original {
                let stream = doc
                    .get_object(*streams.last().unwrap())
                    .unwrap()
                    .as_stream()
                    .unwrap();
                let content = stream
                    .decompressed_content()
                    .unwrap_or_else(|_| stream.content.clone());
                let ops = lopdf::content::Content::decode(&content).unwrap();
                crosses += ops
                    .operations
                    .iter()
                    .filter(|op| op.operator == "S")
                    .count();
            }
        }
        assert_eq!(
            crosses,
            *marks.get(student["id"].as_str().unwrap()).unwrap_or(&0)
        );
    }
}

#[tokio::test]
async fn period_zip_exports_all_students_with_selected_period_and_permissions() {
    let (app, _) = fixture().await;
    let mut admin = Client::new(app.clone());
    let mut outsider = Client::new(app.clone());
    let mut shared = Client::new(app.clone());
    admin
        .call(
            "/auth/register",
            "POST",
            registration(
                "zip-admin@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invitation = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"ZIP colleague"}),
            200,
        )
        .await;
    let colleague = shared
        .call(
            "/auth/register",
            "POST",
            registration(
                "zip-teacher@example.test",
                invitation["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    let class = admin
        .call(
            "/classrooms",
            "POST",
            json!({"name":"ZIP-Klasse","school_year":"2026/27"}),
            200,
        )
        .await;
    let prefix = format!("/classrooms/{}", class["id"].as_str().unwrap());
    let autumn = admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            200,
        )
        .await;
    let autumn_id = autumn["id"].as_str().unwrap();
    let archive_path = format!("{prefix}/periods/{autumn_id}/zip");
    outsider.call(&archive_path, "GET", Value::Null, 401).await;
    shared.call(&archive_path, "GET", Value::Null, 404).await;
    admin.call(&archive_path, "GET", Value::Null, 400).await;
    admin
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Mia Müller","Mia Müller","../Jonas\\Test","Amira Yılmaz"]}),
            200,
        )
        .await;
    let workspace = admin.get(&format!("{prefix}/workspace")).await;
    let students = workspace["students"].as_array().unwrap();
    let first = &pdf::catalog().criteria[0].id;
    let last = &pdf::catalog().criteria.last().unwrap().id;
    admin
        .call(
            &format!("{prefix}/periods/{autumn_id}/assessments"),
            "PATCH",
            json!({"changes":[
                {"student_id":students[0]["id"],"criterion_id":first,"value":"yes"},
                {"student_id":students[0]["id"],"criterion_id":last,"value":"yes"},
                {"student_id":students[1]["id"],"criterion_id":first,"value":"no"},
                {"student_id":students[3]["id"],"criterion_id":last,"value":"yes"}
            ]}),
            204,
        )
        .await;
    let winter = admin
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Winter"}),
            200,
        )
        .await;
    let winter_path = format!("{prefix}/periods/{}/zip", winter["id"].as_str().unwrap());
    let other = admin
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Other","school_year":"2026/27"}),
            200,
        )
        .await;
    admin
        .call(
            &format!(
                "/classrooms/{}/periods/{autumn_id}/zip",
                other["id"].as_str().unwrap()
            ),
            "GET",
            Value::Null,
            404,
        )
        .await;
    admin
        .call(
            &format!("{prefix}/periods/nonexistent/zip"),
            "GET",
            Value::Null,
            404,
        )
        .await;
    admin
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"zip-teacher@example.test"}),
            204,
        )
        .await;
    let bytes = shared
        .raw(&archive_path, "GET", Value::Null, 200, true)
        .await;
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../output/pdf");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("qa-period.zip"), &bytes).unwrap();
    let marks = HashMap::from([
        (students[0]["id"].as_str().unwrap().to_owned(), 2),
        (students[3]["id"].as_str().unwrap().to_owned(), 1),
    ]);
    assert_period_archive(bytes, students, &marks, "Herbst");
    assert_period_archive(
        admin.raw(&winter_path, "GET", Value::Null, 200, true).await,
        students,
        &HashMap::new(),
        "Winter",
    );
    admin
        .call(
            &format!("{prefix}/teachers/{}", colleague["id"].as_str().unwrap()),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    shared.call(&archive_path, "GET", Value::Null, 404).await;
}

#[test]
fn achievement_dates_validate_real_calendar_days() {
    for date in ["2024-02-29", "2000-02-29", "2026-10-28", "2026-01-31"] {
        assert!(valid_date(date), "{date}");
    }
    for date in [
        "2026-02-29",
        "1900-02-29",
        "2026-04-31",
        "2026-13-01",
        "2026-00-01",
        "2026-01-00",
        "0000-01-01",
        "26-10-28",
        "2026/10/28",
        "２０２６-10-28",
        "",
    ] {
        assert!(!valid_date(date), "{date}");
    }
}

#[tokio::test]
async fn achievement_dates_survive_saves_and_copies_and_validate_atomically() {
    let (app, db) = fixture().await;
    let mut client = Client::new(app);
    client
        .call(
            "/auth/register",
            "POST",
            registration(
                "dates@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let class = client
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Dates","school_year":"2026/27"}),
            200,
        )
        .await;
    let prefix = format!("/classrooms/{}", class["id"].as_str().unwrap());
    client
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Mia Müller","Jonas Schmidt"]}),
            200,
        )
        .await;
    let period = client
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Herbst"}),
            200,
        )
        .await;
    let pid = period["id"].as_str().unwrap();
    let workspace = client.get(&format!("{prefix}/workspace")).await;
    let students = workspace["students"].as_array().unwrap();
    let student = &students[0]["id"];
    let second = &students[1]["id"];
    let criterion = &pdf::catalog().criteria[0].id;
    let endpoint = format!("{prefix}/periods/{pid}/assessments");
    let change = json!({"student_id":student,"criterion_id":criterion,"value":"yes"});
    client
        .call(&endpoint, "PATCH", json!({"changes":[change]}), 204)
        .await;
    let today: String = sqlx::query_scalar("SELECT date('now')")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(
        client.get(&format!("{prefix}/workspace")).await["assessments"][0]["achieved_on"],
        today
    );
    let dated = json!({"student_id":student,"criterion_id":criterion,"value":"yes","achieved_on":"2026-09-15"});
    client
        .call(&endpoint, "PATCH", json!({"changes":[dated]}), 204)
        .await;
    client
        .call(&endpoint, "PATCH", json!({"changes":[change]}), 204)
        .await;
    assert_eq!(
        client.get(&format!("{prefix}/workspace")).await["assessments"][0]["achieved_on"],
        "2026-09-15"
    );
    for (value, date) in [("yes", "2026-02-29"), ("no", "2026-10-01")] {
        client.call(&endpoint, "PATCH", json!({"changes":[
            {"student_id":second,"criterion_id":criterion,"value":"yes","achieved_on":"2026-10-01"},
            {"student_id":student,"criterion_id":criterion,"value":value,"achieved_on":date}
        ]}), 400).await;
        assert_eq!(
            client.get(&format!("{prefix}/workspace")).await["assessments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    let copy = client
        .call(
            &format!("{prefix}/periods"),
            "POST",
            json!({"name":"Winter","source_period_id":pid}),
            200,
        )
        .await;
    let copy_id = copy["id"].as_str().unwrap();
    assert_eq!(
        client
            .get(&format!("{prefix}/workspace?period_id={copy_id}"))
            .await["assessments"][0]["achieved_on"],
        "2026-09-15"
    );
    client.call(&format!("{prefix}/periods/{copy_id}/assessments"), "PATCH", json!({"changes":[
        {"student_id":student,"criterion_id":criterion,"value":"yes","achieved_on":"2026-10-20"}
    ]}), 204).await;
    assert_eq!(
        client
            .get(&format!("{prefix}/workspace?period_id={pid}"))
            .await["assessments"][0]["achieved_on"],
        "2026-09-15"
    );
    client
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":student,"criterion_id":criterion,"value":"no"}]}),
            204,
        )
        .await;
    assert!(
        client
            .get(&format!("{prefix}/workspace?period_id={pid}"))
            .await["assessments"][0]["achieved_on"]
            .is_null()
    );
    client
        .call(&endpoint, "PATCH", json!({"changes":[change]}), 204)
        .await;
    assert_eq!(
        client
            .get(&format!("{prefix}/workspace?period_id={pid}"))
            .await["assessments"][0]["achieved_on"],
        today
    );
    client
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":student,"criterion_id":criterion,"value":null}]}),
            204,
        )
        .await;
    assert_eq!(
        client
            .get(&format!("{prefix}/workspace?period_id={pid}"))
            .await["assessments"],
        json!([])
    );
}

#[tokio::test]
async fn legacy_assessments_receive_their_last_recorded_date() {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .in_memory(true)
                .foreign_keys(false),
        )
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("INSERT INTO math_assessments(classroom_id,period_id,student_id,criterion_id,value,updated_by,updated_at) VALUES('c','p','s','yes','yes','t',unixepoch('2026-09-15 12:34:56')),('c','p','s','no','no','t',unixepoch('2026-09-16 12:34:56'))").execute(&db).await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/0003_achievement_dates.sql"))
        .execute(&db)
        .await
        .unwrap();
    let yes: Option<String> =
        sqlx::query_scalar("SELECT achieved_on FROM math_assessments WHERE value='yes'")
            .fetch_one(&db)
            .await
            .unwrap();
    let no: Option<String> =
        sqlx::query_scalar("SELECT achieved_on FROM math_assessments WHERE value='no'")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(yes.as_deref(), Some("2026-09-15"));
    assert!(no.is_none());
}

#[tokio::test]
async fn attendance_is_daily_shared_atomic_and_independent_of_periods() {
    let (app, db) = fixture().await;
    let mut owner = Client::new(app.clone());
    let mut editor = Client::new(app.clone());
    let mut anonymous = Client::new(app.clone());
    let teacher = owner
        .call(
            "/auth/register",
            "POST",
            registration(
                "attendance-owner@example.test",
                "fixture-setup-token-at-least-24-characters",
            ),
            200,
        )
        .await;
    let invite = owner
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Attendance editor"}),
            200,
        )
        .await;
    editor
        .call(
            "/auth/register",
            "POST",
            registration(
                "attendance-editor@example.test",
                invite["token"].as_str().unwrap(),
            ),
            200,
        )
        .await;
    let class = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Attendance class","school_year":"2026/27"}),
            200,
        )
        .await;
    let class = class["id"].as_str().unwrap();
    let prefix = format!("/classrooms/{class}");
    owner
        .call(
            &format!("{prefix}/students"),
            "POST",
            json!({"names":["Same Name","Same Name","Unmarked Child"]}),
            200,
        )
        .await;
    let workspace = owner.get(&format!("{prefix}/workspace")).await;
    assert_eq!(workspace["periods"], json!([]));
    let students = workspace["students"].as_array().unwrap();
    let s0 = students[0]["id"].as_str().unwrap();
    let s1 = students[1]["id"].as_str().unwrap();
    let endpoint = format!("{prefix}/attendance/2026-10-28");
    let yesterday = format!("{prefix}/attendance/2026-10-27");
    assert_eq!(owner.get(&endpoint).await, json!([]));
    anonymous.call(&endpoint, "GET", Value::Null, 401).await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    let changes = json!({"changes":[{"student_id":s0,"status":"present"},{"student_id":s1,"status":"absent"}]});
    anonymous
        .call(&endpoint, "PATCH", changes.clone(), 401)
        .await;
    editor.call(&endpoint, "PATCH", changes.clone(), 404).await;
    owner
        .raw(&endpoint, "PATCH", changes.clone(), 403, false)
        .await;
    owner.call(&endpoint, "PATCH", changes, 204).await;
    let records = owner.get(&endpoint).await;
    assert_eq!(records.as_array().unwrap().len(), 2);
    assert_eq!(
        records
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["student_id"] == s0)
            .unwrap()["status"],
        "present"
    );
    assert_eq!(records[0]["updated_by"], teacher["id"]);
    assert!(records[0]["updated_at"].is_i64());
    assert_eq!(owner.get(&yesterday).await, json!([]));
    for date in [
        "2026-02-29",
        "2026-04-31",
        "0000-01-01",
        "2026-1-01",
        "not-a-date",
    ] {
        let invalid = format!("{prefix}/attendance/{date}");
        owner.call(&invalid, "GET", Value::Null, 400).await;
        owner
            .call(
                &invalid,
                "PATCH",
                json!({"changes":[{"student_id":s0,"status":"present"}]}),
                400,
            )
            .await;
    }
    assert_eq!(
        owner.get(&format!("{prefix}/attendance/2024-02-29")).await,
        json!([])
    );
    owner
        .call(&endpoint, "PATCH", json!({"changes":[]}), 400)
        .await;
    owner.call(&endpoint, "PATCH", json!({"changes":[{"student_id":s0,"status":"absent"},{"student_id":"unknown","status":"present"}]}), 400).await;
    owner.call(&endpoint, "PATCH", json!({"changes":[{"student_id":s0,"status":"absent"},{"student_id":s1,"status":"late"}]}), 400).await;
    owner.call(&endpoint, "PATCH", json!({"changes":[{"student_id":s0,"status":"absent"},{"student_id":s0,"status":null}]}), 400).await;
    assert_eq!(owner.get(&endpoint).await, records);
    let other = owner
        .call(
            "/classrooms",
            "POST",
            json!({"name":"Other class","school_year":"2026/27"}),
            200,
        )
        .await;
    let other_endpoint = format!(
        "/classrooms/{}/attendance/2026-10-28",
        other["id"].as_str().unwrap()
    );
    assert_eq!(owner.get(&other_endpoint).await, json!([]));
    owner
        .call(
            &other_endpoint,
            "PATCH",
            json!({"changes":[{"student_id":s0,"status":"present"}]}),
            400,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/teachers"),
            "POST",
            json!({"email":"attendance-editor@example.test"}),
            204,
        )
        .await;
    assert_eq!(editor.get(&endpoint).await, records);
    editor
        .call(
            &yesterday,
            "PATCH",
            json!({"changes":[{"student_id":s0,"status":"absent"}]}),
            204,
        )
        .await;
    assert_eq!(owner.get(&endpoint).await, records);
    editor.call(&endpoint, "PATCH", json!({"changes":[{"student_id":s0,"status":"absent"},{"student_id":s1,"status":null}]}), 204).await;
    let corrected = owner.get(&endpoint).await;
    assert_eq!(corrected.as_array().unwrap().len(), 1);
    assert_eq!(corrected[0]["status"], "absent");
    let editor_id = editor.get("/auth/me").await["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(corrected[0]["updated_by"], editor_id);
    // Rebuilding the router reads the same persisted records.
    let state = AppState {
        db: db.clone(),
        backups: None,
        secure_cookie: true,
        setup_hash: None,
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let mut reloaded = Client::new(routes(state));
    reloaded.cookie = owner.cookie.clone();
    assert_eq!(reloaded.get(&endpoint).await, corrected);
    owner
        .call(
            &format!("{prefix}/teachers/{editor_id}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    editor.call(&endpoint, "GET", Value::Null, 404).await;
    editor
        .call(
            &endpoint,
            "PATCH",
            json!({"changes":[{"student_id":s0,"status":"present"}]}),
            404,
        )
        .await;
    owner
        .call(
            &format!("{prefix}/students/{s0}"),
            "PATCH",
            json!({"name":"Renamed child"}),
            204,
        )
        .await;
    assert_eq!(owner.get(&endpoint).await, corrected);
    owner
        .call(
            &format!("{prefix}/students/{s0}"),
            "DELETE",
            Value::Null,
            204,
        )
        .await;
    assert_eq!(owner.get(&endpoint).await, json!([]));
    assert_eq!(owner.get(&yesterday).await, json!([]));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attendance")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

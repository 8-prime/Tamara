use super::*;

async fn users() -> (Client, Client, SqlitePool, String, String) {
    let (app, pool) = fixture().await;
    let mut admin = Client::new(app.clone());
    let a = admin
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
    let invitation = admin
        .call(
            "/admin/invitations",
            "POST",
            json!({"label":"Teacher"}),
            200,
        )
        .await;
    let mut user = Client::new(app);
    let u = user
        .call(
            "/auth/register",
            "POST",
            registration("user@example.test", invitation["token"].as_str().unwrap()),
            200,
        )
        .await;
    (
        admin,
        user,
        pool,
        a["id"].as_str().unwrap().into(),
        u["id"].as_str().unwrap().into(),
    )
}

#[tokio::test]
async fn accounts_are_admin_only_and_last_admin_is_protected() {
    let (mut admin, mut user, pool, aid, uid) = users().await;
    user.call("/admin/accounts", "GET", Value::Null, 403).await;
    user.call(
        &format!("/admin/accounts/{aid}"),
        "PATCH",
        json!({"is_admin":true}),
        403,
    )
    .await;
    user.call(
        &format!("/admin/accounts/{aid}/deletion"),
        "GET",
        Value::Null,
        403,
    )
    .await;
    user.call(
        &format!("/admin/accounts/{aid}/password-reset"),
        "POST",
        Value::Null,
        403,
    )
    .await;
    user.call(
        &format!("/admin/accounts/{aid}"),
        "DELETE",
        json!({"password":"long-test-password-123","owned_classroom_ids":[]}),
        403,
    )
    .await;
    let list = admin.get("/admin/accounts").await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    assert!(!list.to_string().contains("password"));
    admin
        .call(
            "/auth/me",
            "DELETE",
            json!({"password":"long-test-password-123","owned_classroom_ids":[]}),
            400,
        )
        .await;
    admin
        .call(
            &format!("/admin/accounts/{aid}"),
            "PATCH",
            json!({"is_admin":false}),
            400,
        )
        .await;
    admin
        .call(
            &format!("/admin/accounts/{uid}"),
            "PATCH",
            json!({"is_admin":true}),
            204,
        )
        .await;
    assert_eq!(user.get("/auth/me").await["is_admin"], true);
    user.call(
        &format!("/admin/accounts/{aid}"),
        "PATCH",
        json!({"is_admin":false}),
        204,
    )
    .await;
    admin.call("/admin/accounts", "GET", Value::Null, 403).await;
    admin
        .call(
            "/auth/me",
            "DELETE",
            json!({"password":"long-test-password-123","owned_classroom_ids":[]}),
            204,
        )
        .await;
    admin.call("/auth/me", "GET", Value::Null, 401).await;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM teachers WHERE id=?")
        .bind(aid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(exists, 0);
}

#[tokio::test]
async fn account_deletion_removes_owned_data_and_anonymizes_shared_records_atomically() {
    let (mut admin, mut user, pool, aid, uid) = users().await;
    let db = Database::new(pool.clone());
    db.create_class("owned", "Owned class", "2026/27", &uid)
        .await
        .unwrap();
    db.create_class("shared", "Other teacher's class", "2026/27", &aid)
        .await
        .unwrap();
    sqlx::query("INSERT INTO classroom_teachers VALUES('owned',?,'editor'),('shared',?,'editor')")
        .bind(&aid)
        .bind(&uid)
        .execute(&pool)
        .await
        .unwrap();
    // Exercise every teacher FK and cross-reference, including classroom cascades.
    for class in ["owned", "shared"] {
        let student = format!("p-{class}");
        let subject = format!("s-{class}");
        let lesson = format!("l-{class}");
        let timetable = format!("tt-{class}");
        sqlx::query("INSERT INTO students(id,classroom_id,name) VALUES(?,?,'Pupil')")
            .bind(&student)
            .bind(class)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO subjects(id,classroom_id,name) VALUES(?,?,'Math')")
            .bind(&subject)
            .bind(class)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO periods(id,classroom_id,name,created_by) VALUES(?,?,'Period',?)")
            .bind(format!("period-{class}"))
            .bind(class)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO math_assessments(classroom_id,period_id,student_id,criterion_id,value,updated_by) SELECT ?,?,?,id,'yes',? FROM math_criteria LIMIT 1").bind(class).bind(format!("period-{class}")).bind(&student).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO timetable_entries(id,teacher_id,classroom_id,subject_id,weekday,starts_at,ends_at,valid_from,valid_until) VALUES(?,?,?,?,1,'08:00','09:00','2026-11-03','2026-11-03')").bind(&timetable).bind(&uid).bind(class).bind(&subject).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lessons(id,timetable_entry_id,teacher_id,classroom_id,subject_id,scheduled_date,lesson_date,starts_at,ends_at) VALUES(?,?,?,?,?,'2026-11-03','2026-11-03','08:00','09:00')").bind(&lesson).bind(&timetable).bind(&uid).bind(class).bind(&subject).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lesson_performances(lesson_id,classroom_id,student_id,rating,updated_by) VALUES(?,?,?,3,?)").bind(&lesson).bind(class).bind(&student).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO student_notes(id,classroom_id,student_id,content,created_by,updated_by,lesson_id) VALUES(?,?,?,'Keep classroom record',?,?,?)").bind(format!("n-{class}")).bind(class).bind(&student).bind(&uid).bind(&uid).bind(&lesson).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO attendance(classroom_id,student_id,attendance_date,status,updated_by) VALUES(?,?,'2026-11-03','present',?)").bind(class).bind(&student).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO homework(id,classroom_id,subject_id,title,due_date,created_by) VALUES(?,?,?,'Homework','2026-11-03',?)").bind(format!("h-{class}")).bind(class).bind(&subject).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO homework_submissions(homework_id,classroom_id,student_id,updated_by) VALUES(?,?,?,?)").bind(format!("h-{class}")).bind(class).bind(&student).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO class_tasks(id,classroom_id,lesson_id,title,completion_mode,created_by) VALUES(?,?,?,'Task','per_pupil',?)").bind(format!("t-{class}")).bind(class).bind(&lesson).bind(&uid).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO class_task_pupils(task_id,classroom_id,student_id,updated_by) VALUES(?,?,?,?)").bind(format!("t-{class}")).bind(class).bind(&student).bind(&uid).execute(&pool).await.unwrap();
    }
    user.call(
        "/conferences/private",
        "PUT",
        json!({"date":"2026-11-03","content":"private","classroom_id":null,"title":null}),
        200,
    )
    .await;
    let reset = admin
        .call(
            &format!("/admin/accounts/{uid}/password-reset"),
            "POST",
            Value::Null,
            200,
        )
        .await;
    let preview = user.get("/auth/me/deletion").await;
    assert_eq!(preview["owned_classrooms"][0]["id"], "owned");
    let deletion = json!({"password":"long-test-password-123","owned_classroom_ids":["owned"]});
    user.call(
        "/auth/me",
        "DELETE",
        json!({"password":"wrong","owned_classroom_ids":["owned"]}),
        403,
    )
    .await;
    user.call(
        "/auth/me",
        "DELETE",
        json!({"password":"long-test-password-123","owned_classroom_ids":[]}),
        409,
    )
    .await;
    assert_eq!(user.get("/classrooms").await.as_array().unwrap().len(), 2);
    // A failed transaction preserves all records, even after classroom deletes.
    sqlx::raw_sql("CREATE TRIGGER prevent_test_deletion BEFORE DELETE ON teachers BEGIN SELECT RAISE(ABORT,'test rollback'); END;").execute(&pool).await.unwrap();
    user.call("/auth/me", "DELETE", deletion.clone(), 500).await;
    assert_eq!(user.get("/classrooms").await.as_array().unwrap().len(), 2);
    sqlx::raw_sql("DROP TRIGGER prevent_test_deletion;")
        .execute(&pool)
        .await
        .unwrap();
    user.call("/auth/me", "DELETE", deletion, 204).await;
    user.call("/auth/me", "GET", Value::Null, 401).await;
    for table in ["teachers", "sessions", "conferences", "password_resets"] {
        let column = if table == "teachers" {
            "id"
        } else {
            "teacher_id"
        };
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE {column}=?"))
                .bind(&uid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    let shared = admin.get("/classrooms/shared/workspace").await;
    assert_eq!(shared["students"].as_array().unwrap().len(), 1);
    let note = admin.get("/classrooms/shared/notes").await;
    assert!(note[0]["created_by"].is_null());
    let lesson = admin.get("/lessons/l-shared").await;
    assert!(lesson["teacher_id"].is_null());
    let references: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(references, 0);
    admin
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":reset["token"],"password":"new-long-password-123"}),
            400,
        )
        .await;
}

#[tokio::test]
async fn reset_links_expire_replace_previous_links_and_revoke_all_sessions() {
    let (mut admin, mut user, pool, _, uid) = users().await;
    let mut another = Client::new(user.app.clone());
    another
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"long-test-password-123"}),
            200,
        )
        .await;
    let issue = format!("/admin/accounts/{uid}/password-reset");
    let old = admin.call(&issue, "POST", Value::Null, 200).await;
    let new = admin.call(&issue, "POST", Value::Null, 200).await;
    let raw = new["token"].as_str().unwrap();
    let stored: String =
        sqlx::query_scalar("SELECT token_hash FROM password_resets WHERE teacher_id=?")
            .bind(&uid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored, hash(raw));
    assert_ne!(stored, raw);
    let mut anonymous = Client::new(user.app.clone());
    anonymous
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":old["token"],"password":"new-long-password-123"}),
            400,
        )
        .await;
    anonymous
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":raw,"password":"short"}),
            400,
        )
        .await;
    assert!(user.get("/auth/me").await["id"].is_string());
    sqlx::query("UPDATE password_resets SET expires_at=unixepoch()-1")
        .execute(&pool)
        .await
        .unwrap();
    anonymous
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":raw,"password":"new-long-password-123"}),
            400,
        )
        .await;
    let valid = admin.call(&issue, "POST", Value::Null, 200).await;
    let input = json!({"token":valid["token"],"password":"new-long-password-123"});
    // Two consumers racing for one link: exactly one wins.
    let mut competitor = Client::new(user.app.clone());
    let (left, right) = tokio::join!(
        anonymous.app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("content-type", "application/json")
                .header("x-tamara-request", "1")
                .body(Body::from(input.to_string()))
                .unwrap()
        ),
        competitor.app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/reset-password")
                .header("content-type", "application/json")
                .header("x-tamara-request", "1")
                .body(Body::from(input.to_string()))
                .unwrap()
        )
    );
    let mut statuses = [
        left.unwrap().status().as_u16(),
        right.unwrap().status().as_u16(),
    ];
    statuses.sort();
    assert_eq!(statuses, [204, 400]);
    user.call("/auth/me", "GET", Value::Null, 401).await;
    another.call("/auth/me", "GET", Value::Null, 401).await;
    competitor
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"long-test-password-123"}),
            401,
        )
        .await;
    competitor
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"new-long-password-123"}),
            200,
        )
        .await;
}

#[tokio::test]
async fn administrators_can_delete_accounts_and_stale_logins_cannot_recreate_sessions() {
    let (mut admin, mut user, pool, _, uid) = users().await;
    let db = Database::new(pool.clone());
    let old = db
        .login_account("user@example.test")
        .await
        .unwrap()
        .unwrap();
    let reset = admin
        .call(
            &format!("/admin/accounts/{uid}/password-reset"),
            "POST",
            Value::Null,
            200,
        )
        .await;
    admin
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":reset["token"],"password":"replacement-password-123"}),
            204,
        )
        .await;
    assert!(
        db.create_session(&uid, "stale-session", &old.password_hash)
            .await
            .is_err()
    );
    admin
        .call(
            &format!("/admin/accounts/{uid}"),
            "DELETE",
            json!({"password":"long-test-password-123","owned_classroom_ids":[]}),
            204,
        )
        .await;
    user.call("/auth/me", "GET", Value::Null, 401).await;
    assert_eq!(
        admin.get("/admin/accounts").await.as_array().unwrap().len(),
        1
    );
}

#[tokio::test]
async fn users_change_their_own_email_after_confirming_their_password() {
    let (mut admin, mut user, _, _, uid) = users().await;
    let path = "/auth/me/email";
    user.call(
        path,
        "PUT",
        json!({"email":"new@example.test","password":"wrong-password-123"}),
        403,
    )
    .await;
    user.call(
        path,
        "PUT",
        json!({"email":"not-an-address","password":"long-test-password-123"}),
        400,
    )
    .await;
    let taken = user
        .call(
            path,
            "PUT",
            json!({"email":"ADMIN@example.test","password":"long-test-password-123"}),
            409,
        )
        .await;
    assert_eq!(
        taken["error"],
        "Diese E-Mail-Adresse wird bereits verwendet."
    );
    let changed = user
        .call(
            path,
            "PUT",
            json!({"email":"  New.Address@Example.test ","password":"long-test-password-123"}),
            200,
        )
        .await;
    assert_eq!(changed["id"], uid.as_str());
    assert_eq!(changed["email"], "new.address@example.test");
    // The current session survives; only the login identifier changes.
    assert_eq!(
        user.get("/auth/me").await["email"],
        "new.address@example.test"
    );
    // Re-saving the unchanged address is allowed.
    user.call(
        path,
        "PUT",
        json!({"email":"new.address@example.test","password":"long-test-password-123"}),
        200,
    )
    .await;
    let mut fresh = Client::new(user.app.clone());
    fresh
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"long-test-password-123"}),
            401,
        )
        .await;
    fresh
        .call(
            "/auth/login",
            "POST",
            json!({"email":"new.address@example.test","password":"long-test-password-123"}),
            200,
        )
        .await;
    let accounts = admin.get("/admin/accounts").await;
    assert!(accounts.to_string().contains("new.address@example.test"));
    assert!(!accounts.to_string().contains("user@example.test"));
}

#[tokio::test]
async fn administrators_change_account_emails() {
    let (mut admin, mut user, _, aid, uid) = users().await;
    user.call(
        &format!("/admin/accounts/{aid}/email"),
        "PUT",
        json!({"email":"taken-over@example.test"}),
        403,
    )
    .await;
    admin
        .call(
            &format!("/admin/accounts/{uid}/email"),
            "PUT",
            json!({"email":"admin@example.test"}),
            409,
        )
        .await;
    admin
        .call(
            "/admin/accounts/missing/email",
            "PUT",
            json!({"email":"nobody@example.test"}),
            404,
        )
        .await;
    // Their own login only changes through the password-confirmed endpoint.
    admin
        .call(
            &format!("/admin/accounts/{aid}/email"),
            "PUT",
            json!({"email":"admin-new@example.test"}),
            403,
        )
        .await;
    let changed = admin
        .call(
            &format!("/admin/accounts/{uid}/email"),
            "PUT",
            json!({"email":"Renamed@Example.test"}),
            200,
        )
        .await;
    assert_eq!(changed["email"], "renamed@example.test");
    assert_eq!(user.get("/auth/me").await["email"], "renamed@example.test");
    let mut fresh = Client::new(user.app.clone());
    fresh
        .call(
            "/auth/login",
            "POST",
            json!({"email":"renamed@example.test","password":"long-test-password-123"}),
            200,
        )
        .await;
}

#[tokio::test]
async fn password_changes_keep_the_current_session_and_end_all_others() {
    let (mut admin, mut user, _, _, uid) = users().await;
    let mut another = Client::new(user.app.clone());
    another
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"long-test-password-123"}),
            200,
        )
        .await;
    let reset = admin
        .call(
            &format!("/admin/accounts/{uid}/password-reset"),
            "POST",
            Value::Null,
            200,
        )
        .await;
    let path = "/auth/me/password";
    user.call(
        path,
        "PUT",
        json!({"current_password":"wrong-password-123","new_password":"changed-password-123"}),
        403,
    )
    .await;
    user.call(
        path,
        "PUT",
        json!({"current_password":"long-test-password-123","new_password":"short"}),
        400,
    )
    .await;
    user.call(
        path,
        "PUT",
        json!({"current_password":"long-test-password-123","new_password":"changed-password-123"}),
        204,
    )
    .await;
    assert!(user.get("/auth/me").await["id"].is_string());
    another.call("/auth/me", "GET", Value::Null, 401).await;
    // A pending reset link no longer works after the owner changed the password.
    another
        .call(
            "/auth/reset-password",
            "POST",
            json!({"token":reset["token"],"password":"reset-password-123"}),
            400,
        )
        .await;
    another
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"long-test-password-123"}),
            401,
        )
        .await;
    another
        .call(
            "/auth/login",
            "POST",
            json!({"email":"user@example.test","password":"changed-password-123"}),
            200,
        )
        .await;
}

#[tokio::test]
async fn account_migration_preserves_existing_data_and_foreign_keys() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = std::borrow::Cow::Owned(
        old.migrations
            .iter()
            .filter(|m| m.version < 11)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();
    sqlx::raw_sql("INSERT INTO teachers(id,name,email,password_hash,is_admin) VALUES('teacher','Existing','existing@example.test','hash',1);
      INSERT INTO classrooms(id,name,school_year) VALUES('class','Existing class','2026/27');
      INSERT INTO classroom_teachers VALUES('class','teacher','owner');
      INSERT INTO students(id,classroom_id,name) VALUES('pupil','class','Existing pupil');
      INSERT INTO subjects(id,classroom_id,name) VALUES('subject','class','Math');
      INSERT INTO periods(id,classroom_id,name,created_by) VALUES('period','class','Period','teacher');
      INSERT INTO homework(id,classroom_id,subject_id,title,due_date,created_by) VALUES('homework','class','subject','Existing homework','2026-11-03','teacher');
      INSERT INTO homework_worksheets(homework_id,name,content) VALUES('homework','sheet.pdf',X'010203');
      INSERT INTO attendance(classroom_id,student_id,attendance_date,status,updated_by) VALUES('class','pupil','2026-11-03','present','teacher');
      INSERT INTO student_notes(id,classroom_id,student_id,content,created_by,updated_by) VALUES('note','class','pupil','Existing note','teacher','teacher');
      INSERT INTO registration_tokens(id,label,token_hash,created_by,expires_at) VALUES('invite','Existing','token','teacher',123);
      INSERT INTO sessions VALUES('session','teacher',9999999999);").execute(&pool).await.unwrap();
    let db = Database::new(pool.clone());
    db.initialize().await.unwrap();
    db.initialize().await.unwrap();
    for table in [
        "teachers",
        "classrooms",
        "classroom_teachers",
        "students",
        "subjects",
        "periods",
        "homework",
        "homework_worksheets",
        "attendance",
        "student_notes",
        "registration_tokens",
        "sessions",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1, "{table}");
    }
    let content: Vec<u8> = sqlx::query_scalar("SELECT content FROM homework_worksheets")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(content, [1, 2, 3]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(
        sqlx::query("INSERT INTO sessions VALUES('invalid','missing',123)")
            .execute(&pool)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[derive(Clone)]
struct LogBuffer(Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogBuffer {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}
#[tokio::test]
async fn security_events_exclude_passwords_tokens_emails_and_note_contents() {
    let buffer = LogBuffer(Arc::new(std::sync::Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(buffer.clone())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let reset_token = async {
        let (mut admin, mut user, _, _, uid) = users().await;
        let reset = admin.call(&format!("/admin/accounts/{uid}/password-reset"),"POST",Value::Null,200).await;
        user.call("/conferences/private","PUT",json!({"date":"2026-11-03","content":"SECRET_NOTE_CONTENT","title":null,"classroom_id":null}),200).await;
        user.call("/auth/login","POST",json!({"email":"user@example.test","password":"SECRET_WRONG_PASSWORD"}),401).await;
        user.call("/admin/accounts","GET",Value::Null,403).await;
        user.call("/auth/me/password","PUT",json!({"current_password":"long-test-password-123","new_password":"SECRET_NEW_PASSWORD"}),204).await;
        user.call("/auth/me/email","PUT",json!({"email":"changed@example.test","password":"SECRET_NEW_PASSWORD"}),200).await;
        user.call("/auth/logout","POST",Value::Null,204).await;
        reset["token"].as_str().unwrap().to_owned()
    }.await;
    let logs = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
    for event in [
        "account_registered",
        "login_failed",
        "password_reset_issued",
        "mutation_completed",
        "request_denied",
        "password_changed",
        "email_changed",
        "logout",
    ] {
        assert!(logs.contains(event), "Missing {event}: {logs}");
    }
    for secret in [
        reset_token.as_str(),
        "SECRET_NOTE_CONTENT",
        "SECRET_WRONG_PASSWORD",
        "long-test-password-123",
        "user@example.test",
        "admin@example.test",
        "changed@example.test",
        "SECRET_NEW_PASSWORD",
    ] {
        assert!(!logs.contains(secret));
    }
}

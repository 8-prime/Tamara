use super::*;
use crate::{
    auth::Credentials,
    error::ErrorKind,
    models::{HomeworkInput, SubmissionChange},
};

#[tokio::test]
async fn auth_service_manages_hashed_sessions_without_http() {
    let (_, pool) = fixture().await;
    let db = Database::new(pool.clone());
    let auth = Auth::new(Some(hash("fixture-setup-token-at-least-24-characters")));
    let credentials: Credentials = serde_json::from_value(registration(
        "auth-service@example.test",
        "fixture-setup-token-at-least-24-characters",
    ))
    .unwrap();
    let session = auth.register(&db, credentials).await.unwrap();
    assert!(session.teacher.is_admin);
    let account = db
        .login_account("auth-service@example.test")
        .await
        .unwrap()
        .unwrap();
    assert!(account.password_hash.starts_with("$argon2id$"));
    assert!(
        !serde_json::to_value(&account.teacher)
            .unwrap()
            .to_string()
            .contains("password")
    );
    let stored: String = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, hash(&session.token));
    assert_ne!(stored, session.token);
    assert_eq!(
        auth.authenticate(&db, Some(&session.token))
            .await
            .unwrap()
            .id,
        session.teacher.id
    );
    assert!(matches!(
        auth.authenticate(&db, None).await,
        Err(crate::error::Error(ErrorKind::Unauthorized, _))
    ));
    assert!(matches!(
        auth.authenticate(&db, Some(&stored)).await,
        Err(crate::error::Error(ErrorKind::Unauthorized, _))
    ));

    auth.logout(&db, Some(&session.token)).await.unwrap();
    assert!(auth.authenticate(&db, Some(&session.token)).await.is_err());
    let login = || Credentials {
        email: "auth-service@example.test".into(),
        password: "long-test-password-123".into(),
        name: None,
        registration_token: None,
    };
    let second = auth.login(&db, login()).await.unwrap();
    assert_ne!(second.token, session.token);
    sqlx::query("UPDATE sessions SET expires_at=unixepoch()-1")
        .execute(&pool)
        .await
        .unwrap();
    assert!(auth.authenticate(&db, Some(&second.token)).await.is_err());
    let mut invalid = login();
    invalid.password = "wrong-password".into();
    let error = auth.login(&db, invalid).await.err().unwrap();
    assert!(matches!(error.0, ErrorKind::Unauthorized));
    let mut unknown = login();
    unknown.email = "unknown@example.test".into();
    let unknown_error = auth.login(&db, unknown).await.err().unwrap();
    assert!(matches!(unknown_error.0, ErrorKind::Unauthorized));
    assert_eq!(error.1, unknown_error.1);
    pool.close().await;
}

#[tokio::test]
async fn repositories_roll_back_submission_batches_without_http() {
    let (_, pool) = fixture().await;
    let db = Database::new(pool.clone());
    let teacher = db
        .register_teacher(
            "Teacher".into(),
            "repository@example.test".into(),
            "unused-password-hash".into(),
            "setup-hash",
            Some("setup-hash"),
        )
        .await
        .unwrap();
    db.create_class("class", "Class", "2026/27", &teacher.id)
        .await
        .unwrap();
    db.add_students("class", vec!["First".into(), "Second".into()])
        .await
        .unwrap();
    db.create_subject("subject", "class", "Math").await.unwrap();
    let homework = db
        .create_homework(
            "class",
            &teacher.id,
            &HomeworkInput {
                subject_id: "subject".into(),
                title: "Practice".into(),
                description: String::new(),
                due_date: "2026-10-30".into(),
            },
        )
        .await
        .unwrap();
    let students = db.students("class").await.unwrap();
    let result = db
        .save_submissions(
            "class",
            &homework,
            &teacher.id,
            vec![
                SubmissionChange {
                    student_id: students[0].id.clone(),
                    handed_in: true,
                },
                SubmissionChange {
                    student_id: "not-assigned".into(),
                    handed_in: true,
                },
            ],
        )
        .await;
    assert!(matches!(
        result,
        Err(crate::error::Error(ErrorKind::BadRequest, _))
    ));
    let (assignment, submissions) = db.homework_detail("class", &homework).await.unwrap();
    assert_eq!(assignment.status, "open");
    assert!(
        submissions
            .iter()
            .all(|submission| submission.handed_in_at.is_none())
    );

    db.save_submissions(
        "class",
        &homework,
        &teacher.id,
        students
            .into_iter()
            .map(|student| SubmissionChange {
                student_id: student.id,
                handed_in: true,
            })
            .collect(),
    )
    .await
    .unwrap();
    let (assignment, submissions) = db.homework_detail("class", &homework).await.unwrap();
    assert_eq!(assignment.status, "auto_closed");
    assert!(
        submissions
            .iter()
            .all(|submission| submission.handed_in_at.is_some())
    );
    pool.close().await;
}

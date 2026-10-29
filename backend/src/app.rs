//! Process configuration, lifecycle and dependency wiring.
use crate::{api, auth, db, state::AppState};
use std::sync::Arc;

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

pub(crate) async fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tamara=info,tower_http=info".into()),
        )
        .init();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/tamara.sqlite".into());
    let (db, database_path) = db::connect(&url).await?;
    let setup_hash = auth::setup_hash(&db).await?;
    let s = AppState {
        backups: Some(Arc::new(db::backup::Backups::new(
            db.clone(),
            &database_path,
        ))),
        db,
        auth: auth::Auth::new(setup_hash),
        secure_cookie: std::env::var("TAMARA_SECURE_COOKIES").as_deref() == Ok("true"),
    };
    let addr = std::env::var("TAMARA_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let (shutdown, receiver) = tokio::sync::watch::channel(false);
    let backup_task = s.backups.as_ref().unwrap().clone().start(receiver);
    tracing::info!("Tamara is ready at http://{addr}");
    let result = axum::serve(listener, api::routes(s))
        .with_graceful_shutdown(shutdown_signal())
        .await;
    shutdown.send(true)?;
    backup_task.await?;
    result?;
    Ok(())
}

//! Authentication and authorization policy, independent of Axum and cookies.
use crate::{
    db::Database,
    error::{Error, ErrorKind, Result, bad, internal},
    models::Teacher,
    util::now,
    validation::{email, text},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Auth {
    setup_hash: Option<String>,
    attempts: Arc<Mutex<HashMap<String, (u32, u64)>>>,
}

pub(crate) struct Session {
    pub(crate) teacher: Teacher,
    pub(crate) token: String,
}

#[derive(Deserialize)]
pub(crate) struct Credentials {
    pub(crate) email: String,
    pub(crate) password: String,
    pub(crate) name: Option<String>,
    pub(crate) registration_token: Option<String>,
}

impl Auth {
    pub(crate) fn new(setup_hash: Option<String>) -> Self {
        Self {
            setup_hash,
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub(crate) async fn authenticate(&self, db: &Database, token: Option<&str>) -> Result<Teacher> {
        let token =
            token.ok_or_else(|| Error(ErrorKind::Unauthorized, "Bitte anmelden.".into()))?;
        db.session_teacher(&hash(token))
            .await?
            .ok_or_else(|| Error(ErrorKind::Unauthorized, "Bitte erneut anmelden.".into()))
    }

    fn throttle(&self, key: &str) -> Result<()> {
        let mut a = self.attempts.lock().map_err(internal)?;
        a.retain(|_, (_, t)| now().saturating_sub(*t) < 600);
        if a.len() > 10000 {
            return Err(Error(
                ErrorKind::TooManyRequests,
                "Bitte später erneut versuchen.".into(),
            ));
        }
        let entry = a.entry(key.into()).or_insert((0, now()));
        if entry.0 >= 10 {
            return Err(Error(
                ErrorKind::TooManyRequests,
                "Zu viele Versuche. Bitte in zehn Minuten erneut versuchen.".into(),
            ));
        }
        entry.0 += 1;
        Ok(())
    }

    pub(crate) async fn register(&self, db: &Database, c: Credentials) -> Result<Session> {
        let e = email(&c.email)?;
        let n = text(c.name.as_deref().unwrap_or(""), "Name", 120)?;
        if c.password.chars().count() < 12 || c.password.len() > 256 {
            return Err(bad(
                "Das Passwort muss mindestens 12 Zeichen lang sein (höchstens 256 Bytes).",
            ));
        }
        self.throttle(&format!("register:{e}"))?;
        let password = c.password;
        let ph = tokio::task::spawn_blocking(move || {
            let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes()).map_err(internal)?;
            Argon2::default()
                .hash_password(password.as_bytes(), &salt)
                .map(|h| h.to_string())
                .map_err(internal)
        })
        .await
        .map_err(internal)??;
        let token = c.registration_token.as_deref().unwrap_or("").trim();
        if token.is_empty() || token.len() > 256 {
            return Err(bad("Bitte einen gültigen Registrierungscode eingeben."));
        }
        let teacher = db
            .register_teacher(n, e, ph, &hash(token), self.setup_hash.as_deref())
            .await?;
        self.session(db, teacher).await
    }

    pub(crate) async fn login(&self, db: &Database, c: Credentials) -> Result<Session> {
        let e = email(&c.email)?;
        if c.password.len() > 256 {
            return Err(bad("Ungültige Zugangsdaten."));
        }
        self.throttle(&format!("login:{e}"))?;
        let account = db.login_account(&e).await?;
        // Verify a dummy hash as well, so unknown emails follow the expensive path.
        const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$EUPjH8QwGhB2Dlmtj1OTpk7+re9ffj+pRYmPo20RUD0";
        let ph = account
            .as_ref()
            .map(|account| account.password_hash.clone())
            .unwrap_or_else(|| DUMMY_HASH.into());
        let ok = tokio::task::spawn_blocking(move || {
            PasswordHash::new(&ph).is_ok_and(|h| {
                Argon2::default()
                    .verify_password(c.password.as_bytes(), &h)
                    .is_ok()
            })
        })
        .await
        .map_err(internal)?;
        if !ok {
            return Err(Error(
                ErrorKind::Unauthorized,
                "E-Mail oder Passwort ist nicht richtig.".into(),
            ));
        }
        let account = account.ok_or_else(|| {
            Error(
                ErrorKind::Unauthorized,
                "E-Mail oder Passwort ist nicht richtig.".into(),
            )
        })?;
        self.attempts
            .lock()
            .map_err(internal)?
            .remove(&format!("login:{e}"));
        self.session(db, account.teacher).await
    }

    async fn session(&self, db: &Database, teacher: Teacher) -> Result<Session> {
        let token = token();
        db.create_session(&teacher.id, &hash(&token)).await?;
        Ok(Session { teacher, token })
    }
    pub(crate) async fn logout(&self, db: &Database, token: Option<&str>) -> Result<()> {
        if let Some(token) = token {
            db.delete_session(&hash(token)).await?;
        }
        Ok(())
    }
    pub(crate) fn require_admin(teacher: &Teacher) -> Result<()> {
        if !teacher.is_admin {
            return Err(Error(
                ErrorKind::Forbidden,
                "Diese Funktion ist nur für Administratoren verfügbar.".into(),
            ));
        }
        Ok(())
    }
    pub(crate) async fn member(&self, db: &Database, class: &str, teacher: &str) -> Result<String> {
        db.classroom_role(class, teacher).await
    }
    pub(crate) async fn owner(&self, db: &Database, class: &str, teacher: &str) -> Result<()> {
        if self.member(db, class, teacher).await? != "owner" {
            return Err(Error(
                ErrorKind::Forbidden,
                "Nur die Klassenleitung kann Freigaben verwalten.".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

pub(crate) fn token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub(crate) async fn setup_hash(
    db: &Database,
) -> std::result::Result<Option<String>, Box<dyn std::error::Error>> {
    if db.teacher_count().await? != 0 {
        return Ok(None);
    }
    let token = std::env::var("TAMARA_SETUP_TOKEN").unwrap_or_else(|_| token());
    if token.len() < 24 {
        return Err("TAMARA_SETUP_TOKEN must contain at least 24 characters".into());
    }
    eprintln!("\nTamara first-run setup token (creates the first administrator):\n{token}\n");
    Ok(Some(hash(&token)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloned_services_share_throttling_and_expired_attempts_are_cleared() {
        let auth = Auth::new(None);
        let clone = auth.clone();
        for _ in 0..10 {
            auth.throttle("login:teacher@example.test").unwrap();
        }
        assert!(matches!(
            clone.throttle("login:teacher@example.test"),
            Err(Error(ErrorKind::TooManyRequests, _))
        ));
        clone.throttle("login:other@example.test").unwrap();
        clone.throttle("register:teacher@example.test").unwrap();
        auth.attempts
            .lock()
            .unwrap()
            .insert("login:teacher@example.test".into(), (10, now() - 600));
        clone.throttle("login:teacher@example.test").unwrap();
    }
}

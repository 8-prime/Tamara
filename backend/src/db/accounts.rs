use super::Database;
use super::usage::{Metric, count};
use crate::{
    error::{Error, ErrorKind, Result, bad, missing},
    models::{Account, Invitation, Teacher},
};

impl Database {
    pub(crate) async fn teacher_count(&self) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM teachers",
        )
        .fetch_one(&self.pool)
        .await?)
    }

    /// The session's teacher, and whether they were already active today (UTC).
    pub(crate) async fn session_teacher(
        &self,
        token_hash: &str,
    ) -> Result<Option<(Teacher, bool)>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            #[sqlx(flatten)]
            teacher: Teacher,
            active_today: bool,
        }
        Ok(sqlx::query_as::<_, Row>(
            "SELECT t.id, t.name, t.email, t.is_admin,
             t.last_active_day IS date('now') AS active_today
             FROM teachers t
             JOIN sessions s ON s.teacher_id=t.id
             WHERE s.token_hash=? AND s.expires_at>unixepoch()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?
        .map(|row| (row.teacher, row.active_today)))
    }

    pub(crate) async fn login_account(&self, email: &str) -> Result<Option<Account>> {
        Ok(sqlx::query_as(
            "SELECT id,name,email,password_hash,is_admin FROM teachers WHERE email=?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn create_session(
        &self,
        teacher: &str,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            "DELETE
             FROM sessions
             WHERE expires_at<=unixepoch()",
        )
        .execute(&self.pool)
        .await?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let inserted = sqlx::query("INSERT INTO sessions(token_hash,teacher_id,expires_at) SELECT ?,id,unixepoch()+604800 FROM teachers WHERE id=? AND password_hash=?")
            .bind(token_hash).bind(teacher).bind(password_hash).execute(&mut *tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(crate::error::Error(
                crate::error::ErrorKind::Unauthorized,
                "Zugangsdaten wurden geändert. Bitte erneut anmelden.".into(),
            ));
        }
        count(&mut *tx, Metric::SessionStarted, 1).await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn delete_session(&self, token_hash: &str) -> Result<()> {
        sqlx::query(
            "DELETE
             FROM sessions
             WHERE token_hash=?",
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn register_teacher(
        &self,
        name: String,
        email: String,
        password_hash: String,
        token_hash: &str,
        setup_hash: Option<&str>,
    ) -> Result<Teacher> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM teachers",
        )
        .fetch_one(&mut *tx)
        .await?;
        let is_admin = count == 0;
        let invitation: Option<String> = if is_admin {
            if setup_hash != Some(token_hash) {
                return Err(bad("Der Einrichtungscode ist nicht gültig."));
            }
            None
        } else {
            Some(
                sqlx::query_scalar::<_, String>(
                    "SELECT id
                     FROM registration_tokens
                     WHERE token_hash=? AND used_by IS NULL
                         AND revoked=0 AND expires_at>unixepoch()",
                )
                .bind(token_hash)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    bad("Der Registrierungscode ist ungültig, abgelaufen oder bereits verwendet.")
                })?,
            )
        };
        let t = Teacher {
            id: crate::util::id(),
            name,
            email,
            is_admin,
        };
        sqlx::query(
            "INSERT INTO teachers(id, name, email, password_hash, is_admin)
             VALUES(?, ?, ?, ?, ?)",
        )
        .bind(&t.id)
        .bind(&t.name)
        .bind(&t.email)
        .bind(password_hash)
        .bind(is_admin)
        .execute(&mut *tx)
        .await?;
        if let Some(invitation) = invitation {
            sqlx::query(
                "UPDATE registration_tokens
                 SET used_by=?, used_at=unixepoch()
                 WHERE id=?",
            )
            .bind(&t.id)
            .bind(invitation)
            .execute(&mut *tx)
            .await?;
        }
        super::usage::count(&mut *tx, Metric::TeacherRegistered, 1).await?;
        tx.commit().await?;
        Ok(t)
    }

    pub(crate) async fn invitations(&self) -> Result<Vec<Invitation>> {
        Ok(sqlx::query_as(
            "SELECT id, label, created_at, expires_at, used_at, revoked
             FROM registration_tokens
             ORDER BY created_at DESC, rowid DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub(crate) async fn create_invitation(
        &self,
        id: &str,
        label: &str,
        token_hash: &str,
        teacher: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO registration_tokens(id, label, token_hash, created_by, expires_at)
             VALUES(?, ?, ?, ?, unixepoch()+604800)",
        )
        .bind(id)
        .bind(label)
        .bind(token_hash)
        .bind(teacher)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn revoke_invitation(&self, id: &str) -> Result<()> {
        if sqlx::query(
            "UPDATE registration_tokens
             SET revoked=1
             WHERE id=? AND used_by IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            != 1
        {
            return Err(missing());
        }
        Ok(())
    }
}

impl Database {
    pub(crate) async fn accounts(&self) -> Result<Vec<crate::models::AccountSummary>> {
        Ok(sqlx::query_as("SELECT id,name,email,is_admin,created_at,(SELECT COUNT(*) FROM classroom_teachers WHERE teacher_id=teachers.id AND role='owner') owned_classrooms FROM teachers ORDER BY name,id").fetch_all(&self.pool).await?)
    }

    pub(crate) async fn account_deletion(
        &self,
        target: &str,
    ) -> Result<crate::models::AccountDeletion> {
        let t: Teacher = sqlx::query_as("SELECT id,name,email,is_admin FROM teachers WHERE id=?")
            .bind(target)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(missing)?;
        let owned_classrooms = sqlx::query_as("SELECT c.id,c.name,(SELECT COUNT(*) FROM students WHERE classroom_id=c.id) student_count FROM classrooms c JOIN classroom_teachers m ON m.classroom_id=c.id WHERE m.teacher_id=? AND m.role='owner' ORDER BY c.id").bind(target).fetch_all(&self.pool).await?;
        Ok(crate::models::AccountDeletion {
            id: t.id,
            name: t.name,
            email: t.email,
            owned_classrooms,
        })
    }

    pub(crate) async fn delete_account(
        &self,
        actor: &str,
        target: &str,
        password_hash: &str,
        confirmed_classes: &[String],
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let allowed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teachers WHERE id=? AND password_hash=? AND (id=? OR is_admin=1))").bind(actor).bind(password_hash).bind(target).fetch_one(&mut *tx).await?;
        if !allowed {
            return Err(crate::error::Error(
                crate::error::ErrorKind::Forbidden,
                "Keine Berechtigung oder Zugangsdaten wurden geändert.".into(),
            ));
        }
        let admin: bool = sqlx::query_scalar("SELECT is_admin FROM teachers WHERE id=?")
            .bind(target)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(missing)?;
        if admin
            && sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teachers WHERE is_admin=1")
                .fetch_one(&mut *tx)
                .await?
                <= 1
        {
            return Err(bad(
                "Das letzte Administrationskonto kann nicht gelöscht werden. Ernenne zuerst eine andere Administration.",
            ));
        }
        let classes: Vec<String> = sqlx::query_scalar("SELECT classroom_id FROM classroom_teachers WHERE teacher_id=? AND role='owner' ORDER BY classroom_id").bind(target).fetch_all(&mut *tx).await?;
        let mut confirmed = confirmed_classes.to_vec();
        confirmed.sort();
        if classes != confirmed {
            return Err(crate::error::Error(
                crate::error::ErrorKind::Conflict,
                "Die Klassen haben sich geändert. Bitte die Löschung erneut prüfen.".into(),
            ));
        }
        // Deferred checking lets SQLite finish all classroom cascades even where
        // a lesson/task/subject references another record in the same classroom.
        sqlx::query("PRAGMA defer_foreign_keys=ON")
            .execute(&mut *tx)
            .await?;
        for class in classes {
            sqlx::query("DELETE FROM classrooms WHERE id=?")
                .bind(class)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM registration_tokens WHERE created_by=? OR used_by=?")
            .bind(target)
            .bind(target)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM teachers WHERE id=?")
            .bind(target)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn set_account_admin(
        &self,
        actor: &str,
        target: &str,
        is_admin: bool,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let allowed: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teachers WHERE id=? AND is_admin=1)")
                .bind(actor)
                .fetch_one(&mut *tx)
                .await?;
        if !allowed {
            return Err(crate::error::Error(
                crate::error::ErrorKind::Forbidden,
                "Nur für Administratoren.".into(),
            ));
        }
        let current: bool = sqlx::query_scalar("SELECT is_admin FROM teachers WHERE id=?")
            .bind(target)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(missing)?;
        if current
            && !is_admin
            && sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM teachers WHERE is_admin=1")
                .fetch_one(&mut *tx)
                .await?
                <= 1
        {
            return Err(bad(
                "Mindestens ein Administrationskonto muss bestehen bleiben.",
            ));
        }
        sqlx::query("UPDATE teachers SET is_admin=? WHERE id=?")
            .bind(is_admin)
            .bind(target)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn issue_password_reset(
        &self,
        actor: &str,
        target: &str,
        token_hash: &str,
    ) -> Result<i64> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let allowed: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teachers WHERE id=? AND is_admin=1)")
                .bind(actor)
                .fetch_one(&mut *tx)
                .await?;
        if !allowed {
            return Err(crate::error::Error(
                crate::error::ErrorKind::Forbidden,
                "Nur für Administratoren.".into(),
            ));
        }
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teachers WHERE id=?)")
            .bind(target)
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Err(missing());
        }
        let expires: i64 = sqlx::query_scalar("INSERT INTO password_resets(token_hash,teacher_id,expires_at) VALUES(?,?,unixepoch()+3600) ON CONFLICT(teacher_id) DO UPDATE SET token_hash=excluded.token_hash,expires_at=excluded.expires_at RETURNING expires_at").bind(token_hash).bind(target).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(expires)
    }

    pub(crate) async fn reset_password(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<String> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let teacher: String = sqlx::query_scalar("DELETE FROM password_resets WHERE token_hash=? AND expires_at>unixepoch() RETURNING teacher_id").bind(token_hash).fetch_optional(&mut *tx).await?.ok_or_else(|| bad("Der Link ist ungültig oder abgelaufen. Bitte einen neuen Link anfordern."))?;
        sqlx::query("UPDATE teachers SET password_hash=? WHERE id=?")
            .bind(password_hash)
            .bind(&teacher)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sessions WHERE teacher_id=?")
            .bind(&teacher)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(teacher)
    }
}

impl Database {
    pub(crate) async fn valid_password_reset(&self, token_hash: &str) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM password_resets WHERE token_hash=? AND expires_at>unixepoch())").bind(token_hash).fetch_one(&self.pool).await?)
    }
}

impl Database {
    /// Changes the login email. Users prove their identity with the password hash
    /// they just confirmed; administrators may change any account.
    pub(crate) async fn change_email(
        &self,
        actor: &str,
        target: &str,
        password_hash: Option<&str>,
        email: &str,
    ) -> Result<Teacher> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let allowed: bool =
            match password_hash {
                Some(password_hash) => sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM teachers WHERE id=? AND id=? AND password_hash=?)",
                )
                .bind(actor)
                .bind(target)
                .bind(password_hash)
                .fetch_one(&mut *tx)
                .await?,
                None => {
                    sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM teachers WHERE id=? AND is_admin=1)",
                    )
                    .bind(actor)
                    .fetch_one(&mut *tx)
                    .await?
                }
            };
        if !allowed {
            return Err(Error(
                ErrorKind::Forbidden,
                "Keine Berechtigung oder Zugangsdaten wurden geändert.".into(),
            ));
        }
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM teachers WHERE email=? AND id<>?)")
                .bind(email)
                .bind(target)
                .fetch_one(&mut *tx)
                .await?;
        if taken {
            return Err(Error(
                ErrorKind::Conflict,
                "Diese E-Mail-Adresse wird bereits verwendet.".into(),
            ));
        }
        let teacher: Teacher = sqlx::query_as(
            "UPDATE teachers SET email=? WHERE id=? RETURNING id,name,email,is_admin",
        )
        .bind(email)
        .bind(target)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(missing)?;
        tx.commit().await?;
        Ok(teacher)
    }

    /// Replaces the password, ends every other session and voids pending reset links.
    pub(crate) async fn change_password(
        &self,
        teacher: &str,
        current_hash: &str,
        new_hash: &str,
        keep_session: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let updated =
            sqlx::query("UPDATE teachers SET password_hash=? WHERE id=? AND password_hash=?")
                .bind(new_hash)
                .bind(teacher)
                .bind(current_hash)
                .execute(&mut *tx)
                .await?;
        if updated.rows_affected() != 1 {
            return Err(Error(
                ErrorKind::Forbidden,
                "Zugangsdaten wurden geändert. Bitte erneut anmelden.".into(),
            ));
        }
        sqlx::query("DELETE FROM sessions WHERE teacher_id=? AND token_hash<>?")
            .bind(teacher)
            .bind(keep_session)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM password_resets WHERE teacher_id=?")
            .bind(teacher)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

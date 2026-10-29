use super::Database;
use crate::{
    error::{Result, bad, missing},
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

    pub(crate) async fn session_teacher(&self, token_hash: &str) -> Result<Option<Teacher>> {
        Ok(sqlx::query_as::<_, Teacher>(
            "SELECT t.id, t.name, t.email, t.is_admin
             FROM teachers t
             JOIN sessions s ON s.teacher_id=t.id
             WHERE s.token_hash=? AND s.expires_at>unixepoch()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn login_account(&self, email: &str) -> Result<Option<Account>> {
        Ok(sqlx::query_as(
            "SELECT id,name,email,password_hash,is_admin FROM teachers WHERE email=?",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub(crate) async fn create_session(&self, teacher: &str, token_hash: &str) -> Result<()> {
        sqlx::query(
            "DELETE
             FROM sessions
             WHERE expires_at<=unixepoch()",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO sessions(token_hash,teacher_id,expires_at) VALUES(?,?,unixepoch()+604800)",
        )
        .bind(token_hash)
        .bind(teacher)
        .execute(&self.pool)
        .await?;
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

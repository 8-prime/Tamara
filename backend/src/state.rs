use crate::{
    auth::Auth,
    db::{Database, backup::Backups},
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Database,
    pub(crate) auth: Auth,
    pub(crate) backups: Option<Arc<Backups>>,
    pub(crate) secure_cookie: bool,
}

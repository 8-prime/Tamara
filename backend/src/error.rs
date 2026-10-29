//! Application errors are independent of HTTP; the API maps them to responses.
#[derive(Debug)]
pub(crate) enum ErrorKind {
    BadRequest,
    NotFound,
    Internal,
    Conflict,
    Unauthorized,
    Forbidden,
    TooManyRequests,
    Unavailable,
    PayloadTooLarge,
}

#[derive(Debug)]
pub(crate) struct Error(pub(crate) ErrorKind, pub(crate) String);
pub(crate) type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}
impl std::error::Error for Error {}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref d) = e
            && d.is_unique_violation()
        {
            return Self(
                ErrorKind::Conflict,
                "Dieser Eintrag existiert bereits.".into(),
            );
        }
        tracing::error!(error=%e, "database operation failed");
        Self(
            ErrorKind::Internal,
            "Die Daten konnten nicht gespeichert werden.".into(),
        )
    }
}
pub(crate) fn bad(s: &str) -> Error {
    Error(ErrorKind::BadRequest, s.into())
}
pub(crate) fn missing() -> Error {
    Error(
        ErrorKind::NotFound,
        "Nicht gefunden oder kein Zugriff.".into(),
    )
}
pub(crate) fn internal(e: impl std::fmt::Display) -> Error {
    tracing::error!(error=%e);
    Error(
        ErrorKind::Internal,
        "Ein interner Fehler ist aufgetreten.".into(),
    )
}

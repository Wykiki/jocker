use std::fmt::Display;

/// Alias for a `Result` with the error type [`jocker::Error`].
pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    pub inner_error: InnerError,
    pub debug_context: Vec<String>,
}

impl Error {
    pub fn new(inner_error: InnerError) -> Self {
        Self {
            inner_error,
            debug_context: vec![],
        }
    }

    pub fn with_context<E: Into<Error>>(inner_error: InnerError) -> impl FnOnce(E) -> Self {
        move |src| {
            let err: Error = src.into();
            err.add_context(inner_error.to_string())
        }
    }

    pub fn add_context<T: Into<String>>(mut self, context: T) -> Self {
        self.debug_context.push(context.into());
        self
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.inner_error)?;
        if !self.debug_context.is_empty() {
            write!(f, " With context:")?;
            for (idx, context) in self.debug_context.iter().enumerate() {
                write!(f, "[{}] {}", idx + 1, context)?
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JockerError: {}", self)
    }
}

impl std::error::Error for Error {}

impl<T: Into<InnerError>> From<T> for Error {
    fn from(src: T) -> Self {
        Error {
            inner_error: src.into(),
            debug_context: vec![],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InnerError {
    #[error("cargo error")]
    Cargo,
    #[error("Env error")]
    Env(String),
    #[error("Filesystem error")]
    Filesystem,
    #[error("UTF-8 error")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Lock error")]
    Lock(String),
    #[error("Parse error")]
    Parse(String),
    #[error("ParseIntError error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Process not found error")]
    ProcessNotFound(Vec<String>),
    #[error("ps error")]
    Ps(String),
    #[error("Recursion deepness too high")]
    RecursionDeepnessTooHigh,
    #[error("Recursion loop")]
    RecursionLoop,
    #[error("Stack not found error")]
    StackNotFound(String),
    #[error("Start stage error")]
    Start(String),
    #[error("SystemTime error")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("TryFromInt error")]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error("Var error")]
    Var(#[from] std::env::VarError),

    // External errors
    #[error("pueue error")]
    Pueue(Box<pueue_lib::Error>),
    #[error("redb commit error")]
    RedbCommit(Box<redb::CommitError>),
    #[error("redb database error")]
    RedbDatabase(Box<redb::DatabaseError>),
    #[error("redb storage error")]
    RedbStorage(Box<redb::StorageError>),
    #[error("redb table error")]
    RedbTable(Box<redb::TableError>),
    #[error("redb transaction error")]
    RedbTransaction(Box<redb::TransactionError>),
    #[error("Serde JSON error")]
    SerdeJson(Box<serde_json::Error>),
    #[error("Serde YAML error")]
    Noyalib(Box<noyalib::Error>),
    #[error("URL error")]
    Url(Box<url::ParseError>),
}

impl From<pueue_lib::Error> for InnerError {
    fn from(value: pueue_lib::Error) -> Self {
        Self::Pueue(Box::new(value))
    }
}

impl From<redb::CommitError> for InnerError {
    fn from(value: redb::CommitError) -> Self {
        Self::RedbCommit(Box::new(value))
    }
}

impl From<redb::DatabaseError> for InnerError {
    fn from(value: redb::DatabaseError) -> Self {
        Self::RedbDatabase(Box::new(value))
    }
}

impl From<redb::StorageError> for InnerError {
    fn from(value: redb::StorageError) -> Self {
        Self::RedbStorage(Box::new(value))
    }
}

impl From<redb::TableError> for InnerError {
    fn from(value: redb::TableError) -> Self {
        Self::RedbTable(Box::new(value))
    }
}

impl From<redb::TransactionError> for InnerError {
    fn from(value: redb::TransactionError) -> Self {
        Self::RedbTransaction(Box::new(value))
    }
}

impl From<serde_json::Error> for InnerError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(Box::new(value))
    }
}

impl From<noyalib::Error> for InnerError {
    fn from(value: noyalib::Error) -> Self {
        Self::Noyalib(Box::new(value))
    }
}

impl From<url::ParseError> for InnerError {
    fn from(value: url::ParseError) -> Self {
        Self::Url(Box::new(value))
    }
}

pub fn lock_error(e: impl Display) -> Error {
    Error::new(InnerError::Lock(e.to_string()))
}

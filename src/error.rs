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
    #[error("Lock error")]
    Lock(String),
    #[error("Parse error")]
    Parse(String),
    #[error("Process not found error")]
    ProcessNotFound(Vec<String>),
    #[error("ps error")]
    Ps(String),
    #[error("Stack not found error")]
    StackNotFound(String),
    #[error("Start stage error")]
    Start(String),

    #[error("UTF-8 error")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Notify error")]
    Notify(#[from] notify::Error),
    #[error("ParseIntError error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Serde JSON error")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Serde YAML error")]
    SerdeYaml(#[from] serde_yml::Error),
    #[error("Sqlite error")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SystemTime error")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("TryFromInt error")]
    TryFromInt(#[from] std::num::TryFromIntError),
    #[error("URL error")]
    Url(#[from] url::ParseError),
    #[error("Var error")]
    Var(#[from] std::env::VarError),
}

pub fn lock_error(e: impl Display) -> Error {
    Error::new(InnerError::Lock(e.to_string()))
}

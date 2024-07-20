use std::fmt::Display;

/// Alias for a `Result` with the error type [`rocker::Error`].
pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    pub inner_error: InnerError,
    pub debug_context: Vec<String>,
}

impl Error {
    pub fn with_context<E: Into<Error>>(inner_error: InnerError) -> impl FnOnce(E) -> Error {
        move |src| {
            let err: Error = src.into();
            err.add_context(inner_error.to_string())
        }
    }

    pub fn add_context<T: Into<String>>(mut self, context: T) -> Self {
        self.debug_context.push(context.into());
        self
    }

    // pub fn cast_with_context<E: Into<Error>, S: Into<String>>(
    //     message: S,
    // ) -> impl FnOnce(E) -> Error {
    //     move |src| {
    //         let netwo_err: Error = src.into();
    //         netwo_err.add_context(message)
    //     }
    // }
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
        write!(f, "RockerError: {}", self)
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
    #[error("Cargo error")]
    Cargo,
    #[error("Env error")]
    Env(String),
    #[error("Filesystem error")]
    Filesystem,
    #[error("State IO error")]
    StateIo,

    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Var error")]
    Var(#[from] std::env::VarError),
    #[error("Serde JSON error")]
    SerdeJson(#[from] serde_json::Error),
}

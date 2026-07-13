use std::{fs::File, path::Path};

use color_eyre::eyre::{Context as _, Result};
use tracing::Level;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber to log to a file
///
/// This function initializes the tracing subscriber to log to a file named `tracing.log` in the
/// current directory. The function returns a [`WorkerGuard`] that must be kept alive for the
/// duration of the program to ensure that logs are flushed to the file on shutdown. The logs are
/// written in a non-blocking fashion to ensure that the logs do not block the main thread.
pub(crate) fn init_tracing(log_file: impl AsRef<Path>) -> Result<WorkerGuard> {
    let file_path = log_file.as_ref();
    let file = File::create(file_path).wrap_err(format!(
        "failed to create tracing file {}",
        file_path.as_os_str().display(),
    ))?;
    println!("{}", file_path.as_os_str().display());
    let (non_blocking, guard) = NonBlocking::new(file);

    // By default, the subscriber is configured to log all events with a level of `INFO` or higher,
    // but this can be changed by setting the `RUST_LOG` environment variable.
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(env_filter)
        .init();
    Ok(guard)
}

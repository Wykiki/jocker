//! Unix signal handling for graceful shutdown.
//!
//! Jocker listens for [`SIGINT`], [`SIGTERM`] and [`SIGHUP`] and turns them into a regular quit
//! event, so the normal teardown path runs: the TUI render loop unwinds, the terminal is restored
//! with [`ratatui::restore()`] and the tracing log file is flushed before the process exits with
//! the conventional `128 + signo` status code.
//!
//! # SIGKILL cannot be handled
//!
//! `SIGKILL` (and `SIGSTOP`) are deliberately impossible to catch, block or ignore: the kernel
//! tears the process down without ever scheduling user code. No amount of application code can
//! restore the terminal when jocker is killed with `kill -9`, which means the shell may be left in
//! raw mode and inside the alternate screen buffer. Recover it with:
//!
//! ```sh
//! reset      # or: stty sane
//! ```
//!
//! Prefer `SIGTERM` (plain `kill`) over `SIGKILL` when stopping jocker.
//!
//! # Ctrl+C and raw mode
//!
//! While the TUI is running the terminal is in raw mode, which clears `ISIG`. The terminal driver
//! therefore does *not* translate Ctrl+C into `SIGINT`: it is delivered to the application as an
//! ordinary key event (handled in [`crate::ui`]). A `SIGINT` reaching this module during the TUI
//! session consequently always comes from an external `kill -INT`.

use std::{io, time::Duration};

use tokio::signal::unix::{signal, SignalKind};

/// How long the graceful shutdown path is given to complete before it is forced.
pub(crate) const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_secs(2);

/// A signal that asks jocker to terminate.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum Shutdown {
    /// `SIGINT`, usually an external `kill -INT`.
    Interrupt,
    /// `SIGTERM`, the default signal sent by `kill`.
    Terminate,
    /// `SIGHUP`, the controlling terminal went away.
    Hangup,
}

impl Shutdown {
    /// The POSIX signal number this variant represents.
    const fn signo(self) -> i32 {
        match self {
            Self::Interrupt => 2,
            Self::Terminate => 15,
            Self::Hangup => 1,
        }
    }

    /// The process exit status to report, following the `128 + signo` shell convention.
    ///
    /// That is `130` for `SIGINT`, `143` for `SIGTERM` and `129` for `SIGHUP`.
    pub(crate) const fn exit_code(self) -> i32 {
        128 + self.signo()
    }
}

impl std::fmt::Display for Shutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Interrupt => "SIGINT",
            Self::Terminate => "SIGTERM",
            Self::Hangup => "SIGHUP",
        };
        write!(f, "{name}")
    }
}

/// Resolves as soon as one of [`SIGINT`], [`SIGTERM`] or [`SIGHUP`] is received.
///
/// The signal listeners are registered when this future is first polled and unregistered when it is
/// dropped, so it is safe to call from several places independently.
///
/// # Errors
///
/// Returns an error if the signal handlers cannot be registered with the runtime.
pub(crate) async fn shutdown_signal() -> io::Result<Shutdown> {
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut hangup = signal(SignalKind::hangup())?;

    Ok(tokio::select! {
        _ = interrupt.recv() => Shutdown::Interrupt,
        _ = terminate.recv() => Shutdown::Terminate,
        _ = hangup.recv() => Shutdown::Hangup,
    })
}

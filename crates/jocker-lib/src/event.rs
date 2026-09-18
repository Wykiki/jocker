use tracing::error;

use crate::{common::ProcessState, types::ProcessName};

#[derive(Debug, Clone)]
pub enum JockerAction {
    Log(Vec<ProcessName>),
    Restart(Vec<ProcessName>),
    Start(Vec<ProcessName>),
    Stop(Vec<ProcessName>),
}

#[derive(Debug, Clone)]
pub enum JockerEvent {
    /// State update on a child process
    ProcessStateChange((ProcessName, ProcessState)),
    /// Jocker main process aborted with a reason
    Abort(String),
    // Building(ProcessName),
    // Starting(ProcessName),
    // Running(ProcessName),
    // Stopping(ProcessName),
    // Stopped(ProcessName),
}

pub trait SendOrLog<T> {
    fn send_or_log(&self, event: T) -> impl std::future::Future<Output = ()>;
}

impl<T> SendOrLog<T> for tokio::sync::broadcast::Sender<T> {
    async fn send_or_log(&self, event: T) {
        if let Err(e) = self.send(event) {
            error!("unable to send value {e}");
        }
    }
}

impl<T> SendOrLog<T> for tokio::sync::mpsc::Sender<T> {
    async fn send_or_log(&self, event: T) {
        if let Err(e) = self.send(event).await {
            error!("unable to send value {e}");
        }
    }
}

use std::{path::PathBuf, sync::Arc};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::trace;

use crate::error::Result;
use crate::event::{JockerAction, JockerEvent, SendOrLog};
use crate::start::{Start, StartArgs};
use crate::state::State;
use crate::types::ProcessName;

pub struct JockerChannel {
    action_tx: mpsc::Sender<JockerAction>,
    event_rx: mpsc::Receiver<JockerEvent>,
}

impl JockerChannel {
    pub fn action_tx(&self) -> &mpsc::Sender<JockerAction> {
        &self.action_tx
    }

    pub fn event_rx(&self) -> &mpsc::Receiver<JockerEvent> {
        &self.event_rx
    }

    pub fn into_inner(self) -> (mpsc::Sender<JockerAction>, mpsc::Receiver<JockerEvent>) {
        (self.action_tx, self.event_rx)
    }
}

pub struct Jocker {
    state: Arc<State>,
}

impl Jocker {
    /// Intialize Jocker state.
    /// Spawns a tokio task for action processing.
    pub async fn new(
        refresh: bool,
        stack: Option<String>,
        target_dir: Option<impl Into<PathBuf>>,
    ) -> Result<(Self, JockerChannel)> {
        let state = Arc::new(State::new(refresh, stack, target_dir).await?);
        let (action_tx, action_rx) = mpsc::channel(64);
        let (event_tx, event_rx) = mpsc::channel(64);
        tokio::spawn(receive(state.clone(), action_rx, event_tx));
        Ok((
            Self { state },
            JockerChannel {
                action_tx,
                event_rx,
            },
        ))
    }

    #[deprecated = "should be removed in favor of message interactions"]
    pub fn state(&self) -> Arc<State> {
        self.state.clone()
    }
}

#[derive(Default)]
struct ActionHandles {
    start_handle: Option<JoinHandle<Result<()>>>,
}

async fn receive(
    state: Arc<State>,
    mut action_rx: mpsc::Receiver<JockerAction>,
    event_tx: mpsc::Sender<JockerEvent>,
) {
    let mut handles = ActionHandles::default();
    while let Some(event) = action_rx.recv().await {
        match event {
            JockerAction::Start(processes) => {
                if let Some(handle) = handles.start_handle {
                    handle.abort();
                }
                handles.start_handle =
                    Some(start(state.clone(), event_tx.clone(), processes).await);
            }
            _ => unimplemented!(),
        }
    }
    event_tx
        .send_or_log(JockerEvent::Abort(
            "broken jocker action receive loop".to_owned(),
        ))
        .await
}

pub async fn start(
    state: Arc<State>,
    event_tx: mpsc::Sender<JockerEvent>,
    processes: Vec<ProcessName>,
) -> JoinHandle<Result<()>> {
    trace!("Jocker::start");
    tokio::spawn(async move {
        Start::new(StartArgs { processes }, state.clone())
            .run(event_tx.clone())
            .await
    })
}

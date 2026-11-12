use std::sync::Arc;

use tokio::{sync::mpsc, task::JoinSet};

use crate::{
    common::{Exec, Process, ProcessState},
    error::Result,
    event::{JockerEvent, JockerMessage, SendOrLog},
    state::State,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StopArgs {
    pub kill: bool,
    pub processes: Vec<String>,
}

pub struct Stop {
    args: StopArgs,
    state: Arc<State>,
}

impl Stop {
    pub fn new(args: StopArgs, state: Arc<State>) -> Self {
        Stop { args, state }
    }

    pub async fn run(&self, event_tx: mpsc::Sender<JockerEvent>) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes).await?;
        let mut handles = JoinSet::new();
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, event_tx.clone(), process, self.args.clone()));
        }

        while let Some(res) = handles.join_next().await {
            match res {
                Err(e) => println!("Error while stopping process: {e}"),
                Ok(ok) => {
                    if let Err(ee) = ok {
                        println!("Error while stopping process inner: {ee}")
                    }
                }
            }
        }
        Ok(())
    }
}

impl Exec<()> for Stop {
    async fn exec(&self) -> Result<()> {
        let (tx, _rx) = mpsc::channel(64);
        self.run(tx.clone()).await
    }
}

async fn run(
    state: Arc<State>,
    event_tx: mpsc::Sender<JockerEvent>,
    process: Process,
    args: StopArgs,
) -> Result<()> {
    let process_name = process.name().to_string();
    if process.state == ProcessState::Stopped && process.pid.is_none() {
        event_tx
            .send_or_log(JockerEvent::Message(JockerMessage::ProcessAlreadyStopped(
                process_name,
            )))
            .await;
        return Ok(());
    }
    if let Some(pid) = process.pid {
        event_tx
            .send_or_log(JockerEvent::Message(JockerMessage::ProcessStopping(
                process_name.clone(),
            )))
            .await;
        state.scheduler().stop(pid, args.kill).await?;
    }
    state
        .set_state(&process_name, ProcessState::Stopped)
        .await?;
    state.set_pid(&process_name, None).await?;
    event_tx
        .send_or_log(JockerEvent::ProcessStateChange((
            process.name().to_owned(),
            ProcessState::Stopped,
        )))
        .await;
    Ok(())
}

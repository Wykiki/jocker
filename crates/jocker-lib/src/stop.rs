use std::sync::Arc;

use tokio::task::JoinSet;

use crate::{
    common::{Exec, Process, ProcessState},
    error::Result,
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
}

impl Exec<()> for Stop {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes).await?;
        let mut handles = JoinSet::new();
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, process, self.args.clone()));
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

async fn run(state: Arc<State>, process: Process, args: StopArgs) -> Result<()> {
    let process_name = process.name().to_string();
    if process.state == ProcessState::Stopped {
        println!("Process is already stopped: {process_name}");
        return Ok(());
    }
    if let Some(pid) = process.pid {
        println!("Stopping process {process_name} ...");
        state.scheduler().stop(pid, args.kill).await?;
    }
    state
        .set_state(&process_name, ProcessState::Stopped)
        .await?;
    state.set_pid(&process_name, None).await?;
    println!("Process {process_name} stopped");
    Ok(())
}

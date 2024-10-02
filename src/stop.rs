use std::sync::Arc;

use argh::FromArgs;
use tokio::task::JoinSet;

use crate::{
    command::kill::{kill_parent_and_children, KillArgs, KillSignal},
    common::{Exec, Process, ProcessState},
    error::Result,
    state::State,
};

#[derive(Clone, Debug, FromArgs, PartialEq)]
/// List processes
#[argh(subcommand, name = "stop")]
pub struct StopArgs {
    /// send SIGKILL instead of SIGTERM
    #[argh(switch)]
    kill: bool,
    #[argh(positional)]
    /// filter process to act upon
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

impl Exec for Stop {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
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
    if process.status == ProcessState::Stopped {
        println!("Process is already stopped: {process_name}");
        return Ok(());
    }
    let pid = if let Some(pid) = process.pid {
        pid
    } else {
        println!("Process does not have a pid: {process_name}");
        return Ok(());
    };
    println!("Stopping process {process_name} ...");
    let signal = if args.kill {
        KillSignal::Kill
    } else {
        KillSignal::default()
    };
    kill_parent_and_children(KillArgs { pid, signal })?;
    state.set_status(&process_name, ProcessState::Stopped)?;
    state.set_pid(&process_name, None)?;
    println!("Process {process_name} stopped");
    Ok(())
}

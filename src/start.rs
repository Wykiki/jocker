use std::{
    process::{Command, ExitStatus},
    sync::Arc,
};

use argh::FromArgs;
use fork::{daemon, Fork};

use crate::{
    common::{Exec, Process, ProcessState},
    error::{Error, InnerError, Result},
    state::State,
};

#[derive(FromArgs, PartialEq, Debug)]
/// Start processes
#[argh(subcommand, name = "start")]
pub struct StartArgs {
    #[argh(positional)]
    /// filter process to act upon
    processes: Vec<String>,
}

pub struct Start {
    args: StartArgs,
    state: Arc<State>,
}

impl Start {
    pub fn new(args: StartArgs, state: Arc<State>) -> Self {
        Start { args, state }
    }
}

impl Exec for Start {
    fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
        let mut handles = vec![];
        for process in processes {
            let state = self.state.clone();
            handles.push(tokio::spawn(run(state, process)));
        }

        Ok(())
    }
}

async fn run(state: Arc<State>, process: Process) {
    let binary = process.binary();
    if process.status != ProcessState::Stopped {
        println!("Process is already started: {}", process.name())
    }
    if let Ok(Fork::Child) = daemon(false, false) {
        let build = Command::new("cargo")
            .arg("build")
            .arg(format!("--package={binary}"))
            .output()
            .expect(&format!(
                "Error while running cargo run for process {}",
                process.name()
            ));
        state
            .set_status(process.name(), ProcessState::Building)
            .expect("Cannot recover from error");
        if !build.status.success() {
            panic!(
                "Build for process {} produced exit code {}",
                process.name(),
                build.status
            );
        }
        Command::new("cargo")
            .arg("build")
            .arg(format!("--package={binary}"))
            .output()
            .expect(&format!(
                "Error while running cargo run for process {}",
                process.name()
            ));
    }
}

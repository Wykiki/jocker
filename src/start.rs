use std::{
    process::{exit, Command, Stdio},
    sync::Arc,
};

use argh::FromArgs;
use fork::{fork, Fork};
use tokio::task::JoinSet;

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
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
        let mut handles = JoinSet::new();
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, process));
        }

        while (handles.join_next().await).is_some() {}

        Ok(())
    }
}

async fn run(state: Arc<State>, process: Process) -> Result<()> {
    if process.status != ProcessState::Stopped {
        println!("Process is already started: {}", process.name());
        return Ok(());
    }
    let process_name = process.name().to_string();
    match fork() {
        Ok(Fork::Parent(child_pid)) => state.set_pid(process.name(), child_pid)?,
        Ok(Fork::Child) => {
            if let Err(err) = run_child(state.clone(), process).await {
                state
                    .log(err)
                    .unwrap_or_else(|e| panic!("Unable to log for process {}: {e}", process_name))
            }
            exit(0);
        }
        Err(e) => state.log(format!("Unable to fork: {e}"))?,
    }
    Ok(())
}

async fn run_child(state: Arc<State>, process: Process) -> Result<()> {
    let binary = process.binary();
    state.set_status(process.name(), ProcessState::Building)?;
    let mut build = Command::new("cargo")
        .arg("build")
        .arg(format!("--package={binary}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::with_context(InnerError::Start(
            "Unable to launch build step".to_string(),
        )))?;
    if let Some(stdout) = build.stdout.take() {
        state.log_process(&process, stdout)?;
    } else {
        state.log("Unable to take ownership of build stdout")?;
    }
    if let Some(stderr) = build.stderr.take() {
        state.log_process(&process, stderr)?;
    } else {
        state.log("Unable to take ownership of build stderr")?;
    }
    let build = build.wait()?;
    if !build.success() {
        state.set_status(process.name(), ProcessState::Stopped)?;
        return Err(Error::new(InnerError::Start(format!(
            "Build for process {} produced exit code {}",
            process.name(),
            build
        ))));
    }

    state.set_status(process.name(), ProcessState::Running)?;
    let mut run = Command::new("cargo")
        .arg("run")
        .arg(format!("--package={binary}"))
        .arg("ps")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::with_context(InnerError::Start(
            "Unable to run crate".to_string(),
        )))?;
    if let Some(stdout) = run.stdout.take() {
        state.log_process(&process, stdout)?;
    } else {
        state.log("Unable to take ownership of run stdout")?;
    }
    if let Some(stderr) = run.stderr.take() {
        state.log_process(&process, stderr)?;
    } else {
        state.log("Unable to take ownership of run stderr")?;
    }
    run.wait()?;
    state.set_status(process.name(), ProcessState::Stopped)?;
    Ok(())
}

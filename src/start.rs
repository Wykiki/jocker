use std::{
    process::{exit, Command, Stdio},
    sync::Arc,
};

use argh::FromArgs;
use dotenvy::dotenv_iter;
use fork::{fork, Fork};

use crate::{
    common::{ConfigFile, Exec, Process, ProcessState},
    error::{Error, InnerError, Result},
    state::State,
};

#[derive(Debug, FromArgs, PartialEq)]
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
        let processes = if let Some(rocker_config) = ConfigFile::load()? {
            processes
                .into_iter()
                .map(|mut p| {
                    if let Some(process_config) = rocker_config.processes.get(p.name()) {
                        p.args.clone_from(&process_config.args);
                        p.env.clone_from(&process_config.env);
                        if let Some(ref binary) = process_config.binary {
                            p.binary.clone_from(binary);
                        }
                    }
                    p
                })
                .collect()
        } else {
            processes
        };
        for process in processes {
            let state = self.state.clone();
            let process_name = process.name().to_string();
            if let Err(e) = run(state, process) {
                println!("Error while starting process {process_name}: {e}")
            }
        }

        Ok(())
    }
}

fn run(state: Arc<State>, process: Process) -> Result<()> {
    if process.status != ProcessState::Stopped {
        println!("Process is already started: {}", process.name());
        return Ok(());
    }
    let process_name = process.name().to_string();
    println!("Starting process {process_name} ...");
    match fork() {
        Ok(Fork::Parent(child_pid)) => state.set_pid(process.name(), Some(child_pid))?,
        Ok(Fork::Child) => {
            state.log("Start child")?;
            if let Err(err) = run_child(state.clone(), process) {
                state.log("Child in error")?;
                state
                    .log(err)
                    .unwrap_or_else(|e| panic!("Unable to log for process {}: {e}", process_name))
            }
            state.log("End child")?;
            exit(0);
        }
        Err(e) => state.log(format!("Unable to fork: {e}"))?,
    }
    println!("Process {process_name} started");
    Ok(())
}

fn run_child(state: Arc<State>, process: Process) -> Result<()> {
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

    let mut run = Command::new("cargo");
    run.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("run")
        .arg(format!("--package={binary}"))
        .arg("--");
    for arg in process.args() {
        run.arg(arg);
    }
    if let Ok(dotenv) = dotenv_iter() {
        for (key, val) in dotenv.flatten() {
            run.env(key, val);
        }
    }
    for (key, val) in process.env.iter() {
        run.env(key, val);
    }
    let mut run = run.spawn().map_err(Error::with_context(InnerError::Start(
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

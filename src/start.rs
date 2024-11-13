use std::{
    collections::HashMap,
    process::{exit, Command, Stdio},
    sync::Arc,
};

use argh::FromArgs;
use dotenvy::dotenv_iter;
use fork::{fork, Fork};
use once_cell::sync::OnceCell;
use regex::Regex;

use crate::{
    command::{cargo::Cargo, util::CommandLogger},
    common::{Exec, Process, ProcessState},
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

    async fn build(&self, processes: &[Process]) -> Result<()> {
        let binaries: Vec<&str> = processes.iter().map(|p| p.binary()).collect();
        let cargo_args: Vec<&str> = processes
            .iter()
            .flat_map(|p| p.cargo_args())
            .map(String::as_str)
            .collect();
        match Cargo::build(binaries.as_slice(), cargo_args.as_slice()).await {
            Ok(mut build_process) => {
                build_process.log_to_console().await?;
                let build_exit_status = build_process.wait().await?;

                if !build_exit_status.success() {
                    return Err(Error::new(InnerError::Start(format!(
                        "Build produced exit code {}",
                        build_exit_status
                    ))));
                }
            }
            Err(e) => {
                println!("Error while building crates: {e}");
                for process in processes {
                    self.state
                        .set_status(process.name(), ProcessState::Stopped)?;
                }
            }
        }
        Ok(())
    }

    fn run(&self, process: Process) -> Result<()> {
        if process.status != ProcessState::Stopped && process.status != ProcessState::Building {
            println!("Process is already started: {}", process.name());
            return Ok(());
        }
        let process_name = process.name().to_string();
        println!("Starting process {process_name} ...");
        match fork() {
            Ok(Fork::Parent(child_pid)) => self.state.set_pid(process.name(), Some(child_pid))?,
            Ok(Fork::Child) => {
                if let Err(err) = run_child(self.state.clone(), process) {
                    self.state.log(err).unwrap_or_else(|e| {
                        panic!("Unable to log for process {}: {e}", process_name)
                    })
                }
                exit(0);
            }
            Err(e) => self.state.log(format!("Unable to fork: {e}"))?,
        }
        println!("Process {process_name} started");
        Ok(())
    }
}

impl Exec for Start {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
        for process in &processes {
            self.state
                .set_status(process.name(), ProcessState::Building)?;
        }
        self.build(processes.as_slice()).await?;
        for process in processes {
            let process_name = process.name().to_string();
            if let Err(e) = self.run(process) {
                println!("Error while starting process {process_name}: {e}")
            }
        }

        Ok(())
    }
}

/// It is NOT possible to use tokio in a forked process
fn run_child(state: Arc<State>, process: Process) -> Result<()> {
    let mut env: HashMap<String, String> = HashMap::new();
    if let Ok(dotenv) = dotenv_iter() {
        for (key, val) in dotenv.flatten() {
            env.insert(key, val);
        }
    }
    for (key, val) in process.env.iter() {
        env.insert(key.to_string(), val.to_string());
    }
    let env = env;

    state.set_status(process.name(), ProcessState::Running)?;

    let mut run = Command::new(format!("./target/debug/{}", process.binary()));
    run.stdout(Stdio::piped()).stderr(Stdio::piped());
    for arg in process.args() {
        run.arg(envsubst(arg, &env));
    }
    for (key, val) in env.iter() {
        run.env(key, val);
    }
    state.log(format!("{}: Run command: {:?}", process.name(), run))?;
    let mut run_process = run.spawn().map_err(Error::with_context(InnerError::Start(
        "Unable to run crate".to_string(),
    )))?;
    if let Some(stdout) = run_process.stdout.take() {
        state.log_process(&process, stdout)?;
    } else {
        state.log("Unable to take ownership of run stdout")?;
    }
    if let Some(stderr) = run_process.stderr.take() {
        state.log_process(&process, stderr)?;
    } else {
        state.log("Unable to take ownership of run stderr")?;
    }
    run_process.wait()?;
    state.set_status(process.name(), ProcessState::Stopped)?;
    Ok(())
}

static ENVSUBST_REGEX: OnceCell<Regex> = OnceCell::new();

pub fn envsubst(value: &str, env: &HashMap<String, String>) -> String {
    let re = ENVSUBST_REGEX.get_or_init(|| Regex::new(r"\$\{([a-zA-Z0-9-_:/.\[\]]*)}").unwrap());

    let mut last_range_end = 0;
    let mut ret = "".to_string();
    // We take all captures, replace them by their associated env value, then build a new string
    // keeping the characters outside of placeholders, using captures' ranges.
    for capture in re.captures_iter(value) {
        let (_, [name]) = capture.extract();
        let range = capture
            .get(0)
            .expect("Cannot happen as i == 0 is guaranteed to return Some")
            .range();
        if range.start != 0 {
            ret.push_str(&value[last_range_end..range.start]);
        }
        last_range_end = range.end;
        let split: Vec<&str> = name.split(":-").collect();
        let var_name = split.first().map(|s| s.to_string()).unwrap_or_default();
        let default = split.get(1).map(|s| s.to_string());
        let var_value = env
            .get(&var_name)
            .map(|s| s.to_string())
            .or(default)
            .unwrap_or_default();
        ret.push_str(&var_value);
    }
    if last_range_end != value.len() {
        ret.push_str(&value[last_range_end..value.len()]);
    }
    ret
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::start::envsubst;

    #[test]
    fn test_envsubst() {
        let mut env = HashMap::new();
        assert_eq!(&envsubst("${FOO:-baz}", &env), "baz");
        env.insert("FOO".to_string(), "BAR".to_string());
        assert_eq!(&envsubst("FOO", &env), "FOO");
        assert_eq!(&envsubst("${FOO}", &env), "BAR");
        assert_eq!(&envsubst("${FOO:-baz}", &env), "BAR");
    }
}

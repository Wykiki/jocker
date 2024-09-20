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
}

impl Exec for Start {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
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
    state.log(format!("{}: Process: {:?}", process.name(), &process))?;
    let binary = process.binary();
    state.set_status(process.name(), ProcessState::Building)?;
    let mut build = Command::new("cargo");
    build.arg("build").arg(format!("--package={binary}"));
    for cargo_arg in process.cargo_args() {
        build.arg(envsubst(cargo_arg, &env));
    }
    for (key, val) in env.iter() {
        build.env(key, val);
    }
    let build = build.stdout(Stdio::piped()).stderr(Stdio::piped());
    state.log(format!("{}: Build command: {:?}", process.name(), build))?;
    let mut build = build
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
        .arg(format!("--package={binary}"));
    for cargo_arg in process.cargo_args() {
        run.arg(envsubst(cargo_arg, &env));
    }
    run.arg("--");
    for arg in process.args() {
        run.arg(envsubst(arg, &env));
    }
    for (key, val) in env.iter() {
        run.env(key, val);
    }
    state.log(format!("{}: Run command: {:?}", process.name(), run))?;
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

static ENVSUBST_REGEX: OnceCell<Regex> = OnceCell::new();

fn envsubst(value: &str, env: &HashMap<String, String>) -> String {
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

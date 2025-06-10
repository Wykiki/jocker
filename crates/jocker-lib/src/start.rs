use std::{collections::HashMap, sync::Arc};

use dotenvy::dotenv_iter;
use once_cell::sync::OnceCell;
use regex::Regex;

use crate::{
    command::{cargo::Cargo, util::CommandLogger},
    common::{Exec, Process, ProcessState},
    error::{Error, InnerError, Result},
    state::State,
};

#[derive(Debug, Default, PartialEq)]
pub struct StartArgs {
    pub processes: Vec<String>,
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
        match Cargo::build(
            self.state.get_target_dir(),
            binaries.as_slice(),
            cargo_args.as_slice(),
        )
        .await
        {
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
                        .set_state(process.name(), ProcessState::Stopped)
                        .await?;
                }
            }
        }
        Ok(())
    }

    pub async fn run(&self, process: Process) -> Result<()> {
        if process.state != ProcessState::Stopped && process.state != ProcessState::Building {
            println!("Process is already started: {}", process.name());
            return Ok(());
        }
        let process_name = process.name().to_string();
        println!("Starting process {process_name} ...");
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

        let mut command = vec![];
        command.push(format!("./target/debug/{}", process.binary()));
        for arg in process.args() {
            command.push(envsubst(arg, &env));
        }

        let pid = self
            .state
            .scheduler()
            .start(
                process_name.clone(),
                command.join(" "),
                self.state.get_target_dir().to_path_buf(),
                env,
            )
            .await?;
        self.state
            .set_state(process.name(), ProcessState::Running)
            .await?;
        self.state.set_pid(process.name(), Some(pid)).await?;
        println!("Process {process_name} started");
        Ok(())
    }
}

impl Exec<()> for Start {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes).await?;
        for process in &processes {
            self.state
                .set_state(process.name(), ProcessState::Building)
                .await?;
        }
        self.build(processes.as_slice()).await?;
        for process in processes {
            let process_name = process.name().to_string();
            if let Err(e) = self.run(process).await {
                println!("Error while starting process {process_name}: {e}")
            }
        }

        Ok(())
    }
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

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use pueue_lib::TaskStatus;
use serde::{Deserialize, Serialize};

use crate::{
    config::ConfigProcess,
    error::{Error, InnerError, Result},
    Pid,
};

pub const JOCKER: &str = "jocker";
pub(crate) const MAX_RECURSION_LEVEL: u8 = 10;
pub const JOCKER_ENV_STACK: &str = "JOCKER_STACK";

#[expect(async_fn_in_trait)]
pub trait Exec<T> {
    async fn exec(&self) -> Result<T>;
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Process {
    pub name: String,
    pub binary: String,
    pub state: ProcessState,
    pub pid: Option<Pid>,
    pub args: Vec<String>,
    pub cargo_args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl Process {
    pub fn new(name: &str, binary: &str) -> Process {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            state: ProcessState::Stopped,
            pid: None,
            args: Vec::new(),
            cargo_args: Vec::new(),
            env: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn binary(&self) -> &str {
        &self.binary
    }

    pub fn pid(&self) -> &Option<Pid> {
        &self.pid
    }

    pub fn args(&self) -> &[String] {
        self.args.as_slice()
    }

    pub fn cargo_args(&self) -> &[String] {
        self.cargo_args.as_slice()
    }
}

impl From<(String, ConfigProcess)> for Process {
    fn from(value: (String, ConfigProcess)) -> Self {
        Self {
            binary: value.1.binary.unwrap_or(value.0.clone()),
            name: value.0,
            args: value.1.args,
            cargo_args: value.1.cargo_args,
            env: value.1.env,
            ..Default::default()
        }
    }
}

impl Ord for Process {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.name.cmp(&other.name) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.binary.cmp(&other.binary) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.state.cmp(&other.state) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.pid.cmp(&other.pid) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.args.cmp(&other.args)
    }
}

impl PartialOrd for Process {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub enum ProcessState {
    Stopped,
    Building,
    Running,
    Unknown,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::Stopped
    }
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ProcessState::Stopped => "stopped",
            ProcessState::Building => "building",
            ProcessState::Running => "running",
            ProcessState::Unknown => "unknown",
        };
        write!(f, "{str}")
    }
}

impl From<TaskStatus> for ProcessState {
    fn from(value: TaskStatus) -> Self {
        match value {
            TaskStatus::Running { .. } => Self::Running,
            TaskStatus::Paused { .. } | TaskStatus::Done { .. } => Self::Stopped,
            _ => Self::Unknown,
        }
    }
}

impl TryFrom<String> for ProcessState {
    type Error = Error;

    fn try_from(value: String) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "stopped" => Self::Stopped,
            "building" => Self::Building,
            "running" => Self::Running,
            "unknown" => Self::Unknown,
            _ => Err(Error::new(InnerError::Parse(value)))?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Stack {
    pub name: String,
    pub processes: HashSet<String>,
    pub inherited_processes: HashSet<String>,
}

impl Stack {
    pub fn get_all_processes(&self) -> HashSet<&String> {
        self.processes
            .iter()
            .chain(self.inherited_processes.iter())
            .collect()
    }
}

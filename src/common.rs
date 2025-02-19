use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    io::BufReader,
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, InnerError, Result};

pub const JOCKER: &str = "jocker";

pub type Pid = u32;

pub trait Exec {
    async fn exec(&self) -> Result<()>;
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Process {
    pub name: String,
    pub binary: String,
    pub status: ProcessState,
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
            status: ProcessState::Stopped,
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
        match self.status.cmp(&other.status) {
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

impl TryFrom<ProcessSql> for Process {
    type Error = Error;

    fn try_from(value: ProcessSql) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            binary: value.binary,
            status: value.status.try_into()?,
            pid: value.pid,
            args: serde_json::from_value(value.args)?,
            cargo_args: serde_json::from_value(value.cargo_args)?,
            env: serde_json::from_value(value.env)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub enum ProcessState {
    Stopped,
    Building,
    Running,
    Healthy,
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
            ProcessState::Healthy => "healthy",
        };
        write!(f, "{str}")
    }
}

impl TryFrom<String> for ProcessState {
    type Error = Error;

    fn try_from(value: String) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "stopped" => Self::Stopped,
            "building" => Self::Building,
            "running" => Self::Running,
            "healthy" => Self::Healthy,
            _ => Err(Error::new(InnerError::Parse(value)))?,
        })
    }
}

pub struct ProcessSql {
    pub name: String,
    pub binary: String,
    pub status: String,
    pub pid: Option<u32>,
    pub args: Value,
    pub cargo_args: Value,
    pub env: Value,
}

pub fn tabled_display_option<T: Display>(value: &Option<T>) -> String {
    match value {
        Some(u) => u.to_string(),
        None => "".to_string(),
    }
}

#[derive(Clone, Debug)]
pub struct Stack {
    pub name: String,
    pub processes: HashSet<String>,
    pub inherited_processes: HashSet<String>,
}

// CONFIG

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub default: Option<ConfigDefault>,
    #[serde(default)]
    pub stacks: HashMap<String, ConfigStack>,
    pub processes: HashMap<String, ConfigProcess>,
}

impl ConfigFile {
    pub fn load() -> Result<Option<Self>> {
        let filename = "jocker.yml";
        if !Path::new(&filename).exists() {
            return Ok(None);
        }
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let res = serde_yml::from_reader(reader)?;
        Ok(res)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigDefault {
    pub stack: Option<String>,
    pub process: Option<ConfigProcessDefault>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigProcessDefault {
    #[serde(default)]
    pub cargo_args: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ConfigStack {
    #[serde(default)]
    pub inherits: HashSet<String>,
    #[serde(default)]
    pub processes: HashSet<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigProcess {
    pub binary: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cargo_args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

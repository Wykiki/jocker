use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub const ROCKER: &str = "rocker";

pub trait Exec {
    async fn exec(&self) -> Result<()>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct Process {
    pub name: String,
    pub binary: String,
    pub status: ProcessState,
    pub pid: Option<i32>,
}

impl Process {
    pub fn new(name: &str, binary: &str) -> Process {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            status: ProcessState::Stopped,
            pid: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn binary(&self) -> &str {
        &self.binary
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub enum ProcessState {
    Stopped,
    Building,
    Running,
    Healthy,
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

pub fn tabled_display_option<T: Display>(value: &Option<T>) -> String {
    match value {
        Some(u) => u.to_string(),
        None => "".to_string(),
    }
}

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::error::Result;

pub const ROCKER: &str = "rocker";

pub trait Exec {
    fn exec(&self) -> Result<()>;
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Process {
    pub name: String,
    pub status: ProcessState,
    pub pid: Option<u32>,
}

impl Process {
    pub fn new(name: &str) -> Process {
        Self {
            name: name.to_string(),
            status: ProcessState::Stopped,
            pid: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Deserialize, Serialize)]
// #[serde(rename_all = "snake_case")]
pub enum ProcessState {
    Stopped,
    Running,
    Healthy,
}

impl Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ProcessState::Stopped => "stopped",
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

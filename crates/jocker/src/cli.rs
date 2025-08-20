use std::{fmt::Display, path::PathBuf};

use clap::{command, Args, Parser, Subcommand};
use jocker_lib::{
    common::ProcessState,
    logs::LogsArgs,
    ps::{PsArgs, PsOutput},
    start::StartArgs,
    stop::StopArgs,
};
use tabled::Tabled;

#[derive(Parser, PartialEq, Debug)]
#[command(version, about, long_about = None)]
/// Top-level command.
pub struct Cli {
    /// verbosity level
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,

    /// whether to trigger a hard refresh
    #[arg(short, long)]
    pub refresh: bool,

    /// which stack to use
    #[arg(short, long, env = "JOCKER_STACK")]
    pub stack: Option<String>,

    /// in which folder to execute action
    #[arg(short, long)]
    pub target_directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, PartialEq, Debug)]
pub enum Commands {
    Ui(UiArgs),
    Clean(CleanArgsCli),
    Logs(LogsArgsCli),
    Ps(PsArgsCli),
    Start(StartArgsCli),
    Stop(StopArgsCli),
}

#[derive(Debug, PartialEq, Args)]
/// First subcommand.
pub struct UiArgs {}

#[derive(Debug, Clone, PartialEq, Args)]
/// Clean jocker state and resources
pub struct CleanArgsCli {}

#[derive(Debug, Clone, PartialEq, Args)]
/// Start processes
pub struct LogsArgsCli {
    /// whether to follow logs or not
    #[arg(short, long)]
    pub follow: bool,
    /// prepend each line with its process name
    #[arg(short, long)]
    pub process_prefix: bool,
    /// only show new log entries
    #[arg(short, long)]
    pub tail: bool,
    /// filter process to act upon
    #[arg(env = "JOCKER_PROCESSES")]
    pub processes: Vec<String>,
}

impl From<LogsArgsCli> for LogsArgs {
    fn from(value: LogsArgsCli) -> Self {
        Self {
            follow: value.follow,
            process_prefix: value.process_prefix,
            tail: value.tail,
            processes: value.processes,
        }
    }
}

#[derive(Debug, PartialEq, Args)]
/// List processes
pub struct PsArgsCli {
    /// filter process to act upon
    #[arg(env = "JOCKER_PROCESSES")]
    pub processes: Vec<String>,
}

impl From<PsArgsCli> for PsArgs {
    fn from(value: PsArgsCli) -> Self {
        Self {
            processes: value.processes,
        }
    }
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
pub struct PsOutputCli {
    name: String,
    state: ProcessState,
    #[tabled(display_with = "tabled_display_option")]
    pid: Option<usize>,
}

impl From<PsOutput> for PsOutputCli {
    fn from(value: PsOutput) -> Self {
        Self {
            name: value.name,
            state: value.state,
            pid: value.pid,
        }
    }
}

#[derive(Debug, PartialEq, Args)]
/// Start processes
pub struct StartArgsCli {
    /// filter process to act upon
    #[arg(env = "JOCKER_PROCESSES")]
    pub processes: Vec<String>,
}

impl From<StartArgsCli> for StartArgs {
    fn from(value: StartArgsCli) -> Self {
        Self {
            processes: value.processes,
        }
    }
}

#[derive(Debug, PartialEq, Args)]
/// List processes
pub struct StopArgsCli {
    /// send SIGKILL instead of SIGTERM
    #[arg(short, long)]
    pub kill: bool,
    /// filter process to act upon
    #[arg(env = "JOCKER_PROCESSES")]
    pub processes: Vec<String>,
}

impl From<StopArgsCli> for StopArgs {
    fn from(value: StopArgsCli) -> Self {
        Self {
            kill: value.kill,
            processes: value.processes,
        }
    }
}

pub fn tabled_display_option<T: Display>(value: &Option<T>) -> String {
    match value {
        Some(u) => u.to_string(),
        None => "".to_string(),
    }
}

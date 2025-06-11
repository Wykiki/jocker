use std::fmt::Display;

use argh::FromArgs;

use jocker_lib::{
    common::ProcessState,
    logs::LogsArgs,
    ps::{PsArgs, PsOutput},
    start::StartArgs,
    stop::StopArgs,
};
use tabled::Tabled;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct Cli {
    /// whether to trigger a hard refresh
    #[argh(switch)]
    pub refresh: bool,

    /// which stack to use
    #[argh(option)]
    pub stack: Option<String>,

    /// in which folder to execute action
    #[argh(option)]
    pub target_directory: Option<String>,

    #[argh(subcommand)]
    pub sub_command: CliSubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum CliSubCommand {
    Ui(UiArgs),
    Clean(CleanArgsCli),
    Logs(LogsArgsCli),
    Ps(PsArgsCli),
    Start(StartArgsCli),
    Stop(StopArgsCli),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "ui")]
pub struct UiArgs {}

#[derive(Clone, Debug, FromArgs, PartialEq)]
/// Clean jocker state and resources
#[argh(subcommand, name = "clean")]
pub struct CleanArgsCli {}

#[derive(Clone, Debug, FromArgs, PartialEq)]
/// Start processes
#[argh(subcommand, name = "logs")]
pub struct LogsArgsCli {
    /// whether to follow logs or not
    #[argh(switch, short = 'f')]
    pub follow: bool,
    /// prepend each line with its process name
    #[argh(switch, short = 'p')]
    pub process_prefix: bool,
    /// only show new log entries
    #[argh(switch, short = 't')]
    pub tail: bool,
    /// filter process to act upon
    #[argh(positional)]
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

#[derive(Debug, FromArgs, PartialEq)]
/// List processes
#[argh(subcommand, name = "ps")]
pub struct PsArgsCli {
    #[argh(positional)]
    /// filter process to act upon
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

#[derive(Debug, FromArgs, PartialEq)]
/// Start processes
#[argh(subcommand, name = "start")]
pub struct StartArgsCli {
    #[argh(positional)]
    /// filter process to act upon
    pub processes: Vec<String>,
}

impl From<StartArgsCli> for StartArgs {
    fn from(value: StartArgsCli) -> Self {
        Self {
            processes: value.processes,
        }
    }
}

#[derive(Clone, Debug, FromArgs, PartialEq)]
/// List processes
#[argh(subcommand, name = "stop")]
pub struct StopArgsCli {
    /// send SIGKILL instead of SIGTERM
    #[argh(switch)]
    pub kill: bool,
    #[argh(positional)]
    /// filter process to act upon
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

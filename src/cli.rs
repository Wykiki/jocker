use argh::FromArgs;

use crate::{logs::LogsArgs, ps::PsArgs, start::StartArgs, stop::StopArgs};

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct Cli {
    /// whether to trigger a hard refresh
    #[argh(switch)]
    pub refresh: bool,

    /// which stack to use
    #[argh(option)]
    pub stack: Option<String>,

    #[argh(subcommand)]
    pub sub_command: CliSubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum CliSubCommand {
    Ui(UiArgs),
    Logs(LogsArgs),
    Ps(PsArgs),
    Start(StartArgs),
    Stop(StopArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "ui")]
pub struct UiArgs {}

use argh::FromArgs;

use crate::{logs::LogsArgs, ps::PsArgs, refresh::RefreshArgs, start::StartArgs, stop::StopArgs};

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct Cli {
    /// whether to trigger a hard refresh
    #[argh(switch)]
    pub refresh: bool,

    #[argh(subcommand)]
    pub sub_command: CliSubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum CliSubCommand {
    Ui(UiArgs),
    Logs(LogsArgs),
    Ps(PsArgs),
    Refresh(RefreshArgs),
    Start(StartArgs),
    Stop(StopArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "ui")]
pub struct UiArgs {}

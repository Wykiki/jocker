use argh::FromArgs;

use crate::{ps::PsArgs, refresh::RefreshArgs, start::StartArgs};

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
pub struct Cli {
    #[argh(subcommand)]
    pub sub_command: CliSubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub enum CliSubCommand {
    Ui(UiArgs),
    Ps(PsArgs),
    Refresh(RefreshArgs),
    Start(StartArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "ui")]
pub struct UiArgs {}

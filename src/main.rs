mod cli;
mod common;
mod error;
mod export_info;
mod logs;
mod ps;
mod refresh;
mod start;
mod state;
mod stop;

use core::panic;
use std::sync::Arc;

use cli::{Cli, CliSubCommand};
use common::Exec;
use logs::Logs;
use ps::Ps;
use refresh::{Refresh, RefreshArgs};
use start::Start;
use state::State;
use stop::Stop;

use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        panic!("platform not supported: windows");
    }
    let cli: Cli = argh::from_env();
    let state = Arc::new(State::new()?);
    Refresh::new(RefreshArgs { hard: cli.refresh }, state.clone())
        .exec()
        .await?;
    match cli.sub_command {
        CliSubCommand::Logs(args) => Logs::new(args, state.clone()).exec().await,
        CliSubCommand::Ps(args) => Ps::new(args, state.clone()).exec().await,
        CliSubCommand::Start(args) => Start::new(args, state.clone()).exec().await,
        CliSubCommand::Stop(args) => Stop::new(args, state.clone()).exec().await,
        _ => panic!(),
    }
}

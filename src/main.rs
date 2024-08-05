mod cli;
mod common;
mod error;
mod export_info;
mod ps;
mod refresh;
mod start;
mod state;
mod stop;

use core::panic;
use std::sync::Arc;

use cli::{Cli, CliSubCommand};
use common::Exec;
use ps::Ps;
use refresh::Refresh;
use start::Start;
use state::State;

use crate::error::Result;

fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        panic!("platform not supported: windows");
    }
    let cli: Cli = argh::from_env();
    let state = Arc::new(State::new()?);
    match cli.sub_command {
        CliSubCommand::Ps(args) => Ps::new(args, state.clone()).exec(),
        CliSubCommand::Refresh(args) => Refresh::new(args, state.clone()).exec(),
        CliSubCommand::Start(args) => Start::new(args, state.clone()).exec(),
        _ => panic!(),
    }
}

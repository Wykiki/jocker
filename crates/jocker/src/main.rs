mod cli;

use core::panic;
use std::sync::Arc;

use cli::{Cli, CliSubCommand, PsOutputCli};
use jocker_lib::common::Exec;
use jocker_lib::logs::Logs;
use jocker_lib::ps::Ps;
use jocker_lib::start::Start;
use jocker_lib::state::State;
use jocker_lib::stop::Stop;

use jocker_lib::error::Result;
use tabled::settings::Style;
use tabled::Table;

#[tokio::main]
pub async fn main() -> Result<()> {
    if cfg!(target_os = "windows") {
        panic!("platform not supported: windows");
    }
    let cli: Cli = argh::from_env();
    let state = Arc::new(State::new(cli.refresh, cli.stack, cli.target_directory).await?);
    match cli.sub_command {
        CliSubCommand::Logs(args) => Logs::new(args.into(), state.clone()).exec().await?,
        CliSubCommand::Ps(args) => {
            let ps: Vec<PsOutputCli> = Ps::new(args.into(), state.clone())
                .run()?
                .into_iter()
                .map(Into::into)
                .collect();
            let mut table = Table::new(ps);
            table.with(Style::blank());
            println!("{table}");
        }
        CliSubCommand::Start(args) => Start::new(args.into(), state.clone()).exec().await?,
        CliSubCommand::Stop(args) => Stop::new(args.into(), state.clone()).exec().await?,
        _ => panic!(),
    };
    Ok(())
}

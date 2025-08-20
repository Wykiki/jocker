mod cli;
mod shell;
mod ui;

use std::sync::Arc;

use clap::{CommandFactory as _, Parser as _};
use clap_complete::generate;
use cli::{Cli, Commands, PsOutputCli};
use jocker_lib::common::Exec;
use jocker_lib::logs::Logs;
use jocker_lib::ps::Ps;
use jocker_lib::start::Start;
use jocker_lib::state::State;
use jocker_lib::stop::Stop;

use jocker_lib::error::{Error, InnerError, Result};
use tabled::settings::Style;
use tabled::Table;
use ui::Ui;

#[tokio::main]
pub async fn main() -> Result<()> {
    color_eyre::install().expect("Unable to install color_eyre");
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbosity.into())
        .init();
    let state = Arc::new(State::new(cli.refresh, cli.stack, cli.target_directory).await?);
    env_logger::Builder::new()
        .filter_level(cli.verbosity.into())
        .init();
    match cli.command {
        Commands::Clean(_) => {
            Arc::try_unwrap(state)
                .map_err(|_| {
                    Error::new(InnerError::Lock(
                        "Unable to unwrap Arc to clean state".to_owned(),
                    ))
                })?
                .clean()
                .await?
        }
        Commands::Completion(args) => {
            let mut cmd = Cli::command();
            let cmd_name = cmd.get_name().to_string();
            generate(args.shell, &mut cmd, cmd_name, &mut std::io::stdout());
        }
        Commands::Logs(args) => Logs::new(args.into(), state.clone()).exec().await?,
        Commands::Ps(args) => {
            let ps: Vec<PsOutputCli> = Ps::new(args.into(), state.clone())
                .run()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            let mut table = Table::new(ps);
            table.with(Style::blank());
            println!("{table}");
        }
        Commands::Start(args) => Start::new(args.into(), state.clone()).exec().await?,
        Commands::Stop(args) => Stop::new(args.into(), state.clone()).exec().await?,
        Commands::Restart(args) => {
            Stop::new(args.clone().into(), state.clone()).exec().await?;
            Start::new(args.clone().into(), state.clone())
                .exec()
                .await?;
        }
        Commands::Ui => {
            let terminal = ratatui::init();
            let app_result = Ui::new(state.clone()).run(terminal).await;
            ratatui::restore();
            return app_result;
        }
    };
    Ok(())
}

mod cli;
mod shell;
mod tracing;
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

use ::tracing::info;
use tabled::settings::Style;
use tabled::Table;
use ui::Ui;

use crate::tracing::init_tracing;

#[tokio::main]
pub async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let state = Arc::new(
        State::new(cli.refresh, cli.stack, cli.target_directory)
            .await
            .map_err(color_eyre::Report::new)?,
    );
    let _guard = init_tracing(state.get_log_file())?;
    info!("Starting jocker");
    match cli.command {
        Commands::Clean(_) => Arc::into_inner(state)
            .ok_or_else(|| {
                color_eyre::Report::msg("unable to unwrap State's Arc, this should not happen")
            })?
            .clean()
            .await
            .map_err(color_eyre::Report::new)?,
        Commands::Completion(args) => {
            let mut cmd = Cli::command();
            let cmd_name = cmd.get_name().to_string();
            generate(args.shell, &mut cmd, cmd_name, &mut std::io::stdout());
        }
        Commands::Logs(args) => Logs::new(args.into(), state.clone())
            .exec()
            .await
            .map_err(color_eyre::Report::new)?,
        Commands::Ps(args) => {
            let ps: Vec<PsOutputCli> = Ps::new(args.into(), state.clone())
                .run()
                .await
                .map_err(color_eyre::Report::new)?
                .into_iter()
                .map(Into::into)
                .collect();
            let mut table = Table::new(ps);
            table.with(Style::blank());
            println!("{table}");
        }
        Commands::Start(args) => Start::new(args.into(), state.clone())
            .exec()
            .await
            .map_err(color_eyre::Report::new)?,
        Commands::Stop(args) => Stop::new(args.into(), state.clone())
            .exec()
            .await
            .map_err(color_eyre::Report::new)?,
        Commands::Restart(args) => {
            Stop::new(args.clone().into(), state.clone())
                .exec()
                .await
                .map_err(color_eyre::Report::new)?;
            Start::new(args.clone().into(), state.clone())
                .exec()
                .await
                .map_err(color_eyre::Report::new)?;
        }
        Commands::Ui => {
            let terminal = ratatui::init();
            let app_result = Ui::new(state.clone()).await.run(terminal).await;
            ratatui::restore();
            return app_result;
        }
    };
    Ok(())
}

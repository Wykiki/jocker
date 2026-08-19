mod cli;
mod shell;
mod signal;
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

use ::tracing::{info, warn};
use tabled::settings::Style;
use tabled::Table;
use ui::Ui;

use crate::signal::shutdown_signal;
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
    let guard = init_tracing(state.get_log_file())?;
    info!("Starting jocker");

    // Status code to exit with, set only when a termination signal cut the command short.
    let mut exit_code = None;

    match cli.command {
        // The ui owns the terminal, so it handles signals itself: aborting it from the outside
        // could drop the terminal mid-draw and skip `ratatui::restore()`.
        Commands::Ui => {
            let terminal = ratatui::init();
            let result = Ui::spawn(state.clone()).await.run(terminal).await;
            ratatui::restore();
            if let Some(signal) = result? {
                info!("ui stopped by {signal}");
                exit_code = Some(signal.exit_code());
            }
        }
        // Every other command is plain stdout output, so it is enough to stop awaiting it. Note
        // that interrupting `start`/`stop` midway leaves the scheduler in whatever state it had
        // reached, exactly like the default signal disposition would have.
        command => {
            tokio::select! {
                result = run_command(command, state.clone()) => result?,
                signal = shutdown_signal() => {
                    let signal = signal?;
                    warn!("received {signal}, stopping");
                    exit_code = Some(signal.exit_code());
                }
            }
        }
    };

    // Flush the tracing worker before a possible `process::exit`, which skips destructors.
    drop(guard);
    if let Some(exit_code) = exit_code {
        std::process::exit(exit_code);
    }
    Ok(())
}

/// Runs every command but [`Commands::Ui`], which needs to own the terminal.
async fn run_command(command: Commands, state: Arc<State>) -> color_eyre::Result<()> {
    match command {
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
        Commands::Ui => unreachable!("the ui command is handled by the caller"),
    };
    Ok(())
}

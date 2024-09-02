use std::sync::Arc;

use argh::FromArgs;
use tabled::{settings::Style, Table, Tabled};

use crate::{
    common::{tabled_display_option, Exec, Process, ProcessState},
    error::Result,
    state::State,
};

#[derive(Debug, FromArgs, PartialEq)]
/// List processes
#[argh(subcommand, name = "ps")]
pub struct PsArgs {
    #[argh(switch, short = 'a')]
    /// whether to show even non-running processes
    pub all: bool,
    #[argh(positional)]
    /// filter process to act upon
    pub processes: Vec<String>,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
pub struct PsOutput {
    name: String,
    status: ProcessState,
    #[tabled(display_with = "tabled_display_option")]
    pid: Option<u32>,
}

impl From<Process> for PsOutput {
    fn from(value: Process) -> Self {
        Self {
            name: value.name,
            status: value.status,
            pid: value.pid,
        }
    }
}

pub struct Ps {
    args: PsArgs,
    state: Arc<State>,
}

impl Ps {
    pub fn new(args: PsArgs, state: Arc<State>) -> Self {
        Ps { args, state }
    }

    pub fn run(&self) -> Result<Vec<PsOutput>> {
        self.state.refresh()?;
        let mut processes = self.state.filter_processes(&self.args.processes)?;
        processes.sort();
        Ok(processes.into_iter().map(PsOutput::from).collect())
    }
}

impl Exec for Ps {
    async fn exec(&self) -> Result<()> {
        let ps = self.run()?;
        let mut table = Table::new(ps);
        table.with(Style::blank());
        println!("{table}");
        Ok(())
    }
}

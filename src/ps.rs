use argh::FromArgs;
use tabled::{settings::Style, Table, Tabled};

use crate::{
    common::{tabled_display_option, Exec, Process, ProcessState},
    error::Result,
    export_info::SerializedPackage,
    state::State,
};

#[derive(FromArgs, PartialEq, Debug)]
/// List processes
#[argh(subcommand, name = "ps")]
pub struct PsArgs {
    #[argh(switch, short = 'a')]
    /// whether to show even non-running processes
    all: bool,
}

#[derive(Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct PsOutput {
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
    state: State,
}

impl Ps {
    pub fn new(args: PsArgs, state: State) -> Self {
        Ps { args, state }
    }
}

impl Exec for Ps {
    fn exec(&self) -> Result<()> {
        let bins = self.state.get_processes()?.into_iter().map(PsOutput::from);
        let mut table = Table::new(bins);
        table.with(Style::blank());
        println!("{table}");
        Ok(())
    }
}

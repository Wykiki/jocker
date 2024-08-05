use argh::FromArgs;

use crate::{common::Exec, error::Result, state::State};

#[derive(FromArgs, PartialEq, Debug)]
/// Start processes
#[argh(subcommand, name = "start")]
pub struct StartArgs {
    #[argh(positional)]
    /// whether to show even non-running processes
    processes: Vec<String>,
}

// #[derive(Tabled)]
// #[tabled(rename_all = "UPPERCASE")]
// struct StartOutput {
//     name: String,
//     status: ProcessState,
//     #[tabled(display_with = "tabled_display_option")]
//     pid: Option<u32>,
// }
//
// impl From<Process> for StartOutput {
//     fn from(value: Process) -> Self {
//         Self {
//             name: value.name,
//             status: value.status,
//             pid: value.pid,
//         }
//     }
// }

pub struct Start {
    args: StartArgs,
    state: State,
}

impl Start {
    pub fn new(args: StartArgs, state: State) -> Self {
        Start { args, state }
    }
}

impl Exec for Start {
    fn exec(&self) -> Result<()> {
        if self.args.processes.is_empty() {
            self.state.get_processes()?;
        } else {
            self.state.filter_processes(&self.args.processes)?;
        }
        Ok(())
    }
}

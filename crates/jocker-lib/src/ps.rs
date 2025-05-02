use std::sync::Arc;

use crate::{
    common::{Exec, Process, ProcessState},
    error::Result,
    state::State,
};

#[derive(Debug, Default, PartialEq)]
pub struct PsArgs {
    pub processes: Vec<String>,
}

pub struct PsOutput {
    pub name: String,
    pub status: ProcessState,
    pub pid: Option<u32>,
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
        let mut processes = self.state.filter_processes(&self.args.processes)?;
        processes.sort();
        Ok(processes.into_iter().map(PsOutput::from).collect())
    }
}

impl Exec<Vec<PsOutput>> for Ps {
    async fn exec(&self) -> Result<Vec<PsOutput>> {
        self.run()
    }
}

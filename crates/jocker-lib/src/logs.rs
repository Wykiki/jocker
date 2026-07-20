use std::sync::Arc;

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};
use tracing::trace;

use crate::{
    common::{Exec, Process, ProcessState},
    error::{Error, InnerError, Result},
};

use crate::state::State;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub process: String,
    pub line: String,
}

impl LogLine {
    pub fn new(process: impl AsRef<str>, line: impl AsRef<str>) -> Self {
        Self {
            process: process.as_ref().to_owned(),
            line: line.as_ref().to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LogsArgs {
    pub follow: bool,
    pub process_prefix: bool,
    pub tail: bool,
    pub processes: Vec<String>,
}

pub struct Logs {
    args: LogsArgs,
    state: Arc<State>,
}

impl Logs {
    pub fn new(args: LogsArgs, state: Arc<State>) -> Self {
        Logs { args, state }
    }

    pub async fn run(&self) -> Result<(JoinSet<Result<()>>, Receiver<LogLine>)> {
        let processes = self.state.filter_processes(&self.args.processes).await?;
        let mut handles = JoinSet::new();
        let (tx, rx) = mpsc::channel(processes.len() * 2);
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, process, self.args.clone(), tx.clone()));
        }

        Ok((handles, rx))
    }
}

impl Exec<()> for Logs {
    async fn exec(&self) -> Result<()> {
        let (mut handles, mut rx) = self.run().await.unwrap();

        let processes = self.state.filter_processes(&self.args.processes).await?;
        let max_process_name_len = processes.iter().fold(0, |acc, e| {
            if acc < e.name().len() {
                e.name().len()
            } else {
                acc
            }
        });

        while let Some(LogLine {
            process,
            line: text,
        }) = rx.recv().await
        {
            if self.args.process_prefix {
                print!("{process:max_process_name_len$} > ");
            }
            println!("{text}");
        }

        while (handles.join_next().await).is_some() {}

        Ok(())
    }
}

async fn run(
    state: Arc<State>,
    process: Process,
    args: LogsArgs,
    log_tx: Sender<LogLine>,
) -> Result<()> {
    trace!(
        "Start log task for {}, follow = {}, prefix = {}, tail = {}",
        process.name(),
        args.follow,
        args.process_prefix,
        args.tail
    );
    let process_name = process.name();

    if !args.tail {
        state
            .scheduler()
            .logs(
                log_tx,
                process_name,
                process.pid().ok_or_else(|| {
                    Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                        "PID missing for log".to_owned(),
                    )))
                })?,
                None,
                args.follow,
            )
            .await?;
    }

    if !args.follow || process.state == ProcessState::Stopped {
        return Ok(());
    }

    trace!(
        "End log task for {}, follow = {}, prefix = {}, tail = {}",
        process.name(),
        args.follow,
        args.process_prefix,
        args.tail
    );

    Ok(())
}

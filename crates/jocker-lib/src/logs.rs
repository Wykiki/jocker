use std::{
    collections::{BTreeMap, HashMap},
    io::stdout,
    io::Write,
    sync::Arc,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};
use tracing::{error, trace, warn};

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

    pub async fn run(&self) -> Result<(JoinSet<Result<()>>, Receiver<BTreeMap<usize, Vec<u8>>>)> {
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
        let (mut handles, rx) = self.run().await.unwrap();
        let mut log_stream = ReceiverStream::new(rx);

        let processes = self.state.filter_processes(&self.args.processes).await?;
        let max_process_name_len = processes.iter().fold(0, |acc, e| {
            if acc < e.name().len() {
                e.name().len()
            } else {
                acc
            }
        });

        let process_by_pid = self
            .state
            .get_processes()
            .await?
            .into_iter()
            .filter_map(|p| p.pid.map(|pid| (pid, p.name)))
            .collect::<HashMap<_, _>>();
        while let Some(logs_by_pid) = log_stream.next().await {
            for (pid, log_bytes) in logs_by_pid {
                let str = match String::from_utf8(log_bytes) {
                    Ok(str) => str,
                    Err(e) => {
                        error!("unable to read logs for process with pid {pid}: {e}");
                        continue;
                    }
                };
                let lines = str.lines();
                let process_name = if self.args.process_prefix {
                    match process_by_pid.get(&pid) {
                        Some(name) => name.as_str(),
                        None => {
                            warn!("unable to get process name for pid {pid}");
                            ""
                        }
                    }
                } else {
                    ""
                };
                let mut lock = stdout().lock();
                for line in lines {
                    if self.args.process_prefix {
                        if let Err(e) = write!(lock, "{process_name:max_process_name_len$} > ") {
                            warn!("unable to write process name to console: {e}")
                        }
                    }
                    if let Err(e) = writeln!(lock, "{line}") {
                        warn!("unable to write log line to console: {e}")
                    }
                }
            }
        }

        while (handles.join_next().await).is_some() {}

        Ok(())
    }
}

async fn run(
    state: Arc<State>,
    process: Process,
    args: LogsArgs,
    log_tx: Sender<BTreeMap<usize, Vec<u8>>>,
) -> Result<()> {
    trace!(
        "Start log task for {}, follow = {}, prefix = {}, tail = {}",
        process.name(),
        args.follow,
        args.process_prefix,
        args.tail
    );

    if !args.tail {
        state
            .scheduler()
            .logs(
                log_tx,
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

    trace!(
        "End log task for {}, follow = {}, prefix = {}, tail = {}",
        process.name(),
        args.follow,
        args.process_prefix,
        args.tail
    );

    if !args.follow || process.state == ProcessState::Stopped {
        return Ok(());
    }

    Ok(())
}

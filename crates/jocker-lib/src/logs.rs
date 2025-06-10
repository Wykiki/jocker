use std::sync::Arc;

use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};

use crate::{
    common::{Exec, Process, ProcessState},
    error::{Error, InnerError, Result},
};

use crate::state::State;

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

    pub async fn run(&self) -> Result<(JoinSet<Result<()>>, Receiver<String>)> {
        let processes = self.state.filter_processes(&self.args.processes).await?;
        let mut handles = JoinSet::new();
        let max_process_name_len = processes.iter().fold(0, |acc, e| {
            if acc < e.name().len() {
                e.name().len()
            } else {
                acc
            }
        });
        let (tx, rx) = mpsc::channel(processes.len() * 2);
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(
                state,
                process,
                self.args.clone(),
                max_process_name_len,
                tx.clone(),
            ));
        }

        Ok((handles, rx))
    }
}

impl Exec<()> for Logs {
    async fn exec(&self) -> Result<()> {
        let (mut handles, mut rx) = self.run().await.unwrap();

        while let Some(message) = rx.recv().await {
            println!("{message}");
        }

        while (handles.join_next().await).is_some() {}

        Ok(())
    }
}

async fn run(
    state: Arc<State>,
    process: Process,
    args: LogsArgs,
    max_process_name_len: usize,
    log_tx: Sender<String>,
) -> Result<()> {
    let process_name = process.name();
    // get file
    // let path = state.filename_log_process(&process);

    // get pos to end of file
    // let f = File::open(&path).await?;
    let process_prefix = if args.process_prefix {
        format!("{process_name:max_process_name_len$} > ")
    } else {
        "".to_string()
    };
    if !args.tail {
        // let reader = BufReader::new(f);
        // let mut lines = reader.lines();
        state
            .scheduler()
            .logs(
                log_tx,
                &process_prefix,
                process.pid().ok_or_else(|| {
                    Error::new(InnerError::Pueue(pueue_lib::Error::Generic(
                        "PID missing for log".to_owned(),
                    )))
                })?,
                None,
                args.follow,
            )
            .await?;
        // while let Ok(Some(line)) = lines.next_line().await {
        //     log_tx
        //         .send(format!("{process_prefix}{}", line))
        //         .await
        //         .unwrap();
        // }
    }

    if !args.follow || process.state == ProcessState::Stopped {
        return Ok(());
    }

    // set up watcher
    // let mut f = File::open(&path).await?;
    // let mut pos = f.metadata().await?.len();
    // f.seek(SeekFrom::Start(pos)).await?;
    // pos = f.metadata().await?.len();
    // let (tx, rx) = std::sync::mpsc::channel();
    // let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    // watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
    //
    // // watch
    // for res in rx {
    //     match res {
    //         Ok(_event) => {
    //             // ignore any event that didn't change the pos
    //             if f.metadata().await?.len() == pos {
    //                 continue;
    //             }
    //
    //             // read from pos to end of file
    //             f.seek(std::io::SeekFrom::Start(pos)).await?;
    //
    //             // update post to end of file
    //             pos = f.metadata().await?.len();
    //
    //             let reader = BufReader::new(f.try_clone().await?);
    //             let mut lines = reader.lines();
    //             while let Ok(Some(line)) = lines.next_line().await {
    //                 log_tx
    //                     .send(format!("{process_prefix}{}", line,))
    //                     .await
    //                     .unwrap();
    //             }
    //         }
    //         Err(error) => println!("{error:?}"),
    //     }
    // }

    Ok(())
}

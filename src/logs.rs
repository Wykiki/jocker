use std::{
    io::{BufRead, BufReader, Seek, SeekFrom},
    sync::Arc,
};

use argh::FromArgs;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::task::JoinSet;

use crate::{
    common::{Exec, Process},
    error::Result,
    state::State,
};

#[derive(Clone, FromArgs, PartialEq, Debug)]
/// Start processes
#[argh(subcommand, name = "logs")]
pub struct LogsArgs {
    /// whether to follow logs or not
    #[argh(switch, short = 'f')]
    follow: bool,
    /// prepend each line with its process name
    #[argh(switch, short = 'p')]
    process_prefix: bool,
    #[argh(positional)]
    /// filter process to act upon
    processes: Vec<String>,
}

pub struct Logs {
    args: LogsArgs,
    state: Arc<State>,
}

impl Logs {
    pub fn new(args: LogsArgs, state: Arc<State>) -> Self {
        Logs { args, state }
    }
}

impl Exec for Logs {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
        let mut handles = JoinSet::new();
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, process, self.args.clone()));
        }

        while (handles.join_next().await).is_some() {}

        Ok(())
    }
}

async fn run(state: Arc<State>, process: Process, args: LogsArgs) -> Result<()> {
    let process_name = process.name();
    // get file
    let path = state.filename_log_process(&process);

    // get pos to end of file
    let f = std::fs::File::open(&path)?;
    let reader = BufReader::new(f);
    let process_prefix = if args.process_prefix {
        format!("{process_name} > ")
    } else {
        "".to_string()
    };
    for line in reader.lines() {
        println!("{process_prefix}{}", line.unwrap_or("".to_string()));
    }
    if !args.follow {
        return Ok(());
    }

    // set up watcher
    let mut f = std::fs::File::open(&path)?;
    let mut pos = std::fs::metadata(&path)?.len();
    f.seek(SeekFrom::Start(pos)).unwrap();
    pos = f.metadata().unwrap().len();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    // watch
    for res in rx {
        match res {
            Ok(_event) => {
                // ignore any event that didn't change the pos
                if f.metadata()?.len() == pos {
                    continue;
                }

                // read from pos to end of file
                f.seek(std::io::SeekFrom::Start(pos))?;

                // update post to end of file
                pos = f.metadata()?.len();

                let reader = BufReader::new(&f);
                for line in reader.lines() {
                    println!("{process_prefix}{}", line.unwrap());
                }
            }
            Err(error) => println!("{error:?}"),
        }
    }

    Ok(())
}

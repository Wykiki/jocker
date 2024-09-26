use std::{collections::HashSet, fmt::Display, io::BufRead, process::Command, sync::Arc};

use argh::FromArgs;
use tokio::task::JoinSet;

use crate::{
    common::{Exec, Process, ProcessState},
    error::Result,
    state::State,
};

#[derive(Clone, Debug, FromArgs, PartialEq)]
/// List processes
#[argh(subcommand, name = "stop")]
pub struct StopArgs {
    /// send SIGKILL instead of SIGTERM
    #[argh(switch)]
    kill: bool,
    #[argh(positional)]
    /// filter process to act upon
    pub processes: Vec<String>,
}

pub struct Stop {
    args: StopArgs,
    state: Arc<State>,
}

impl Stop {
    pub fn new(args: StopArgs, state: Arc<State>) -> Self {
        Stop { args, state }
    }
}

impl Exec for Stop {
    async fn exec(&self) -> Result<()> {
        let processes = self.state.filter_processes(&self.args.processes)?;
        let mut handles = JoinSet::new();
        for process in processes {
            let state = self.state.clone();
            handles.spawn(run(state, process, self.args.clone()));
        }

        while let Some(res) = handles.join_next().await {
            match res {
                Err(e) => println!("Error while stopping process: {e}"),
                Ok(ok) => match ok {
                    Err(ee) => println!("Error while stopping process inner: {ee}"),
                    _ => (),
                },
            }
        }

        Ok(())
    }
}

async fn run(state: Arc<State>, process: Process, args: StopArgs) -> Result<()> {
    let process_name = process.name().to_string();
    if process.status == ProcessState::Stopped {
        println!("Process is already stopped: {process_name}");
        return Ok(());
    }
    let pid = if let Some(pid) = process.pid {
        pid
    } else {
        println!("Process does not have a pid: {process_name}");
        return Ok(());
    };
    println!("Stopping process {process_name} ...");
    let signal = if args.kill {
        KillSignal::Kill
    } else {
        KillSignal::default()
    };
    kill_parent_and_children(KillArgs { pid, signal })?;
    state.set_status(&process_name, ProcessState::Stopped)?;
    state.set_pid(&process_name, None)?;
    println!("Process {process_name} stopped");
    Ok(())
}

// TODO : Recursion may be needed to salvage childrens of childrens
fn kill_parent_and_children(args: KillArgs) -> Result<()> {
    let mut pids = ps(PsArgs {
        ppid: Some(args.pid),
    })?;
    pids.insert(args.pid);
    for pid in pids {
        kill(KillArgs {
            pid,
            ..args.clone()
        })?;
    }
    Ok(())
}

struct PsArgs {
    ppid: Option<u32>,
}

fn ps(args: PsArgs) -> Result<HashSet<u32>> {
    let mut ps = Command::new("ps");
    if let Some(ppid) = args.ppid {
        ps.arg("--ppid");
        ps.arg(ppid.to_string());
    }
    ps.arg("--no-headers");
    ps.arg("-o");
    ps.arg("pid");
    let output = ps.output()?;
    let mut pids: HashSet<u32> = HashSet::new();
    for line in output.stdout.lines() {
        pids.insert(line?.trim().parse()?);
    }
    Ok(pids)
}

#[derive(Clone)]
enum KillSignal {
    Kill,
    Term,
}

impl Default for KillSignal {
    fn default() -> Self {
        Self::Term
    }
}

impl Display for KillSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Kill => "KILL",
            Self::Term => "TERM",
        };
        write!(f, "{text}")
    }
}

#[derive(Clone)]
struct KillArgs {
    pid: u32,
    signal: KillSignal,
}

fn kill(args: KillArgs) -> Result<()> {
    let mut kill = Command::new("kill");
    kill.arg("-s");
    kill.arg(args.signal.to_string());
    kill.arg(args.pid.to_string());
    kill.status()?;
    Ok(())
}

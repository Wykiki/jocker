use std::fmt::Display;
use std::process::Command;

use crate::error::Result;
use crate::{command::ps::ps, command::ps::PsArgs};

// TODO : Recursion may be needed to salvage childrens of childrens
pub fn kill_parent_and_children(args: KillArgs) -> Result<()> {
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

#[derive(Clone)]
pub enum KillSignal {
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
pub struct KillArgs {
    pub pid: u32,
    pub signal: KillSignal,
}

fn kill(args: KillArgs) -> Result<()> {
    let mut kill = Command::new("kill");
    kill.arg("-s");
    kill.arg(args.signal.to_string());
    kill.arg(args.pid.to_string());
    kill.status()?;
    Ok(())
}

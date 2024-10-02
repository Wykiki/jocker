use std::{collections::HashSet, io::BufRead, process::Command, str::FromStr};

use crate::error::{Error, InnerError, Result};

pub struct PsArgs {
    pub ppid: Option<u32>,
}

struct PsOutput {
    ppid: u32,
    pid: u32,
}

impl FromStr for PsOutput {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();
        Ok(Self {
            ppid: parts
                .next()
                .ok_or_else(|| {
                    Error::new(InnerError::Parse(
                        "Missing ppid part from ps output".to_owned(),
                    ))
                })?
                .parse()?,
            pid: parts
                .next()
                .ok_or_else(|| {
                    Error::new(InnerError::Parse(
                        "Missing pid part from ps output".to_owned(),
                    ))
                })?
                .parse()?,
        })
    }
}

pub fn ps(args: PsArgs) -> Result<HashSet<u32>> {
    let mut ps = Command::new("ps");
    ps.arg("-A");
    ps.arg("-o");
    // Trailing '=' sign is mandatory to avoid printing header row, and it is the only way
    // compatible with both Linux and MacOS
    ps.arg("ppid=,pid=");
    let output = ps.output()?;
    let mut pids: HashSet<u32> = HashSet::new();
    for line in output.stdout.lines() {
        let ps_line = PsOutput::from_str(&line?)?;
        if let Some(ppid) = args.ppid {
            if ps_line.ppid != ppid {
                continue;
            }
        }
        pids.insert(ps_line.pid);
    }
    Ok(pids)
}

#[cfg(test)]
mod tests {
    use std::{process::Command, str::FromStr};

    use crate::{command::ps::ps, command::ps::PsArgs};

    use super::PsOutput;

    #[test]
    fn test_ps_output_from_str() {
        let output = PsOutput::from_str("1234  4123980").unwrap();
        assert_eq!(output.ppid, 1234);
        assert_eq!(output.pid, 4123980);
    }

    #[test]
    fn test_ps_with_username() {
        let mut sleep = Command::new("sleep");
        sleep.arg("10");
        let mut sleep = sleep.spawn().unwrap();
        let pids = ps(PsArgs {
            ppid: Some(std::process::id()),
        })
        .unwrap();
        assert!(!pids.is_empty());
        sleep.kill().unwrap();
    }
}

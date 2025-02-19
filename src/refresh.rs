use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::Path,
    process::Command,
    sync::Arc,
};

use chrono::Utc;

use crate::{
    common::{ConfigFile, ConfigStack, Exec, Process, Stack},
    error::{Error, InnerError, Result},
    export_info::{BinaryPackage, ExportInfoMinimal, SerializedPackage, TargetKind},
    state::State,
};

const MAX_RECURSION_LEVEL: u8 = 10;

#[derive(Debug, PartialEq)]
pub struct RefreshArgs {
    pub hard: bool,
}

pub struct Refresh {
    args: RefreshArgs,
    state: Arc<State>,
}

impl Refresh {
    pub fn new(args: RefreshArgs, state: Arc<State>) -> Self {
        Refresh { args, state }
    }

    fn needs_to_refresh_binaries(&self) -> Result<bool> {
        let elapsed_since_last_update = self.state.get_elapsed_since_last_binaries_update()?;
        let files = ["./Cargo.toml", "./Cargo.lock"];
        for file in files {
            if Path::new(file).exists()
                && File::open(file)?
                    .metadata()?
                    .modified()?
                    .elapsed()?
                    .as_secs()
                    < elapsed_since_last_update
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn needs_to_refresh_config(&self) -> Result<bool> {
        let elapsed_since_last_update = self.state.get_elapsed_since_last_config_update()?;
        let files = ["./jocker.yml", "./jocker.override.yml"];
        for file in files {
            if Path::new(file).exists()
                && File::open(file)?
                    .metadata()?
                    .modified()?
                    .elapsed()?
                    .as_secs()
                    < elapsed_since_last_update
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn fetch_bins() -> Result<Vec<SerializedPackage>> {
        let metadata = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .output()
            .map_err(Error::with_context(InnerError::Cargo))?;
        let info: ExportInfoMinimal = serde_json::from_slice(&metadata.stdout).unwrap();
        let ret = info
            .packages
            .into_iter()
            .filter(|package| {
                package
                    .targets
                    .iter()
                    .filter(|target| {
                        target
                            .kind
                            .iter()
                            .filter(|kind| matches!(kind, TargetKind::Bin))
                            .count()
                            >= 1
                    })
                    .count()
                    >= 1
                    && package.id.scheme().eq("path+file")
            })
            .collect();
        Ok(ret)
    }

    fn refresh(&self) -> Result<()> {
        if self.args.hard || self.needs_to_refresh_binaries()? {
            self.refresh_binaries()?;
            self.state.set_binaries_updated_at(Utc::now())?;
        }
        if self.args.hard || self.needs_to_refresh_config()? {
            self.refresh_processes()?;
            self.refresh_stacks()?;
            self.state.set_config_updated_at(Utc::now())?;
        }
        Ok(())
    }

    fn refresh_binaries(&self) -> Result<()> {
        if !self.args.hard {
            return Ok(());
        }
        let binaries: Vec<BinaryPackage> =
            Self::fetch_bins()?.into_iter().map(Into::into).collect();
        self.state.set_binaries(binaries)?;
        Ok(())
    }

    fn refresh_processes(&self) -> Result<()> {
        let previous_processes: HashMap<String, Process> = self
            .state
            .get_processes()?
            .into_iter()
            .map(|p| (p.name().to_string(), p))
            .collect();
        let processes: Vec<Process> = if let Some(jocker_config) = ConfigFile::load()? {
            let mut processes = vec![];
            let process_defaults = jocker_config.default.and_then(|d| d.process);
            for config_process in jocker_config.processes {
                let mut process: Process = config_process.into();

                if let Some(ref process_defaults) = process_defaults {
                    process
                        .cargo_args
                        .append(&mut process_defaults.cargo_args.clone());
                }
                processes.push(process);
            }
            processes
        } else {
            self.state
                .get_binaries()?
                .into_iter()
                .map(|b| Process::new(b.name(), b.name()))
                .collect()
        };
        let processes: Vec<Process> = processes
            .into_iter()
            .map(|mut p| {
                if let Some(previous_process) = previous_processes.get(p.name()) {
                    p.pid = previous_process.pid;
                    p.status = previous_process.status.clone();
                };
                p
            })
            .collect();
        self.state.set_processes(processes)?;

        Ok(())
    }

    fn refresh_stacks(&self) -> Result<()> {
        let mut default_stack = None;
        let stacks = if let Some(jocker_config) = ConfigFile::load()? {
            if let Some(config_default_stack) = jocker_config.default.and_then(|d| d.stack) {
                default_stack = Some(config_default_stack);
            }
            let mut stacks: HashMap<String, Stack> = HashMap::new();
            let config_stacks = jocker_config.stacks.clone();

            for (stack_name, config_stack) in jocker_config.stacks {
                stacks.insert(
                    stack_name.clone(),
                    Stack {
                        name: stack_name.clone(),
                        processes: config_stack.processes,
                        inherited_processes: Default::default(),
                    },
                );
                let inherited_processes = Self::recurse_inherited_processes(
                    0,
                    &config_stack.inherits,
                    &config_stacks,
                    &mut HashSet::new(),
                    HashSet::new(),
                )?;
                stacks
                    .get_mut(&stack_name)
                    .ok_or_else(|| Error::new(InnerError::StackNotFound(stack_name.to_owned())))
                    .map(|stack| stack.inherited_processes = inherited_processes)?;
            }
            stacks
        } else {
            HashMap::new()
        };
        if let Some(default_stack) = default_stack.as_ref() {
            if !stacks.contains_key(default_stack) {
                return Err(Error::new(InnerError::StackNotFound(
                    default_stack.to_owned(),
                )));
            }
        }
        self.state.set_stacks(stacks.values().cloned().collect())?;
        self.state.set_default_stack(&default_stack)?;

        Ok(())
    }

    fn recurse_inherited_processes(
        recursion_level: u8,
        stack_names: &HashSet<String>,
        stacks: &HashMap<String, ConfigStack>,
        browsed_stacks: &mut HashSet<String>,
        mut inherited_processes: HashSet<String>,
    ) -> Result<HashSet<String>> {
        if recursion_level > MAX_RECURSION_LEVEL {
            return Err(Error::new(InnerError::RecursionDeepnessTooHigh));
        }
        for stack_name in stack_names {
            if !browsed_stacks.insert(stack_name.to_owned()) {
                return Err(Error::new(InnerError::RecursionLoop));
            }
            let stack = stacks
                .get(stack_name)
                .ok_or_else(|| Error::new(InnerError::StackNotFound(stack_name.to_owned())))?;
            inherited_processes.extend(stack.processes.clone().into_iter());
            inherited_processes = Self::recurse_inherited_processes(
                recursion_level + 1,
                &stack.inherits,
                stacks,
                browsed_stacks,
                inherited_processes,
            )?;
        }
        Ok(inherited_processes)
    }
}

impl Exec for Refresh {
    async fn exec(&self) -> Result<()> {
        self.refresh()
    }
}

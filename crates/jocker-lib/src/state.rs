use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{canonicalize, create_dir_all, File},
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use tokio::fs::remove_dir_all;

use crate::{
    command::{
        cargo::{BinaryPackage, Cargo},
        pueue::Pueue,
    },
    common::{Process, ProcessState, Stack, JOCKER, MAX_RECURSION_LEVEL},
    config::{ConfigFile, ConfigStack},
    database::Database,
    error::{lock_error, Error, InnerError, Result},
};

#[derive(Debug, PartialEq)]
pub struct StateArgs {
    pub refresh: bool,
    pub stack: Option<String>,
}

pub struct State {
    project_dir: String,
    target_dir: PathBuf,
    db: Database,
    current_stack: Arc<Mutex<Option<String>>>,
    scheduler: Pueue,
}

impl State {
    pub async fn new(
        refresh: bool,
        stack: Option<String>,
        target_dir: Option<impl Into<PathBuf>>,
    ) -> Result<Self> {
        let target_dir = target_dir.map(Into::into).unwrap_or(canonicalize(".")?);
        let (project_id, project_dir) = Self::get_or_create_state_dir(&target_dir)?;
        let db = Database::new(&project_dir)?;
        let scheduler = Pueue::new(&project_id).await?;
        let state = Self {
            project_dir,
            target_dir,
            db,
            current_stack: Arc::new(Mutex::new(None)),
            scheduler,
        };
        state.refresh(refresh).await?;
        state.set_current_stack(&stack)?;
        Ok(state)
    }

    pub(crate) fn scheduler(&self) -> &Pueue {
        &self.scheduler
    }

    pub async fn clean(self) -> Result<()> {
        remove_dir_all(self.project_dir).await?;
        self.scheduler.clean().await?;
        Ok(())
    }

    pub fn get_elapsed_since_last_binaries_update(&self) -> Result<u64> {
        let date = if let Some(date) = self.db.get_binaries_updated_at()? {
            date
        } else {
            DateTime::UNIX_EPOCH
        };
        Ok(Utc::now()
            .signed_duration_since(date)
            .num_seconds()
            .clamp(0, i64::MAX)
            .try_into()?)
    }

    pub fn get_elapsed_since_last_config_update(&self) -> Result<u64> {
        let date = if let Some(date) = self.db.get_config_updated_at()? {
            date
        } else {
            DateTime::UNIX_EPOCH
        };
        Ok(Utc::now()
            .signed_duration_since(date)
            .num_seconds()
            .clamp(0, i64::MAX)
            .try_into()?)
    }

    pub fn set_binaries_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        self.db.set_binaries_updated_at(date)
    }

    pub fn set_config_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        self.db.set_config_updated_at(date)
    }

    pub fn get_target_dir(&self) -> &Path {
        &self.target_dir
    }

    pub fn get_binaries(&self) -> Result<Vec<BinaryPackage>> {
        let bins_iter = self.db.get_binaries()?;
        let mut binaries = vec![];
        for bin in bins_iter {
            binaries.push(bin.try_into()?);
        }
        Ok(binaries)
    }

    pub fn set_binaries(&self, binaries: &[BinaryPackage]) -> Result<()> {
        self.db.set_binaries(binaries)
    }

    /// Filter processes list based on given process names
    ///
    /// If [`process_names`] is empty, returns all processes
    pub fn filter_processes(&self, process_names: &[String]) -> Result<Vec<Process>> {
        let current_stack = self.get_current_stack()?;
        let expected_processes: Vec<String> = if !process_names.is_empty() {
            process_names.to_owned()
        } else if let Some(stack) = current_stack {
            self.get_stack(&stack)?
                .get_all_processes()
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::with_capacity(0)
        };
        if expected_processes.is_empty() {
            return self.get_processes();
        }
        let processes: Vec<Process> = self
            .get_processes()?
            .into_iter()
            .filter(|process| expected_processes.contains(&process.name))
            .collect();
        if expected_processes.len() != processes.len() {
            let mut process_names: HashSet<String> = process_names.iter().cloned().collect();
            for process in processes {
                process_names.remove(&process.name);
            }
            return Err(Error::new(InnerError::ProcessNotFound(
                process_names.into_iter().collect(),
            )));
        }
        Ok(processes)
    }

    pub fn get_processes(&self) -> Result<Vec<Process>> {
        self.db.get_processes()
    }

    pub fn set_processes(&self, processes: Vec<Process>) -> Result<()> {
        self.db.set_processes(&processes)
    }

    pub fn set_state(&self, process_name: &str, state: ProcessState) -> Result<()> {
        self.db.set_process_state(process_name, state)
    }

    pub fn set_pid(&self, process_name: &str, pid: Option<usize>) -> Result<()> {
        let pid = pid.map(i32::try_from).transpose()?;
        self.db.set_process_pid(process_name, pid)
    }

    pub fn get_current_stack(&self) -> Result<Option<String>> {
        Ok(self.current_stack.lock().map_err(lock_error)?.clone())
    }

    pub fn set_current_stack(&self, stack: &Option<String>) -> Result<()> {
        if let Some(stack) = stack {
            *self.current_stack.lock().map_err(lock_error)? = Some(self.get_stack(stack)?.name);
        } else {
            *self.current_stack.lock().map_err(lock_error)? = self.get_default_stack()?;
        };

        Ok(())
    }

    pub fn get_default_stack(&self) -> Result<Option<String>> {
        self.db.get_default_stack()
    }

    pub fn set_default_stack(&self, stack: &Option<String>) -> Result<()> {
        self.db.set_default_stack(stack)
    }

    pub fn get_stack(&self, stack: &str) -> Result<Stack> {
        self.db.get_stack(stack)
    }

    pub fn set_stacks(&self, stacks: &[Stack]) -> Result<()> {
        self.db.set_stacks(stacks)
    }

    // Refresh

    pub async fn refresh(&self, hard: bool) -> Result<()> {
        let mut scheduled_process = self.scheduler().processes().await?;
        for process in self.get_processes()? {
            if let Some(sp) = scheduled_process.remove(process.name()) {
                self.set_pid(process.name(), Some(sp.0))?;
                self.set_state(process.name(), sp.1.into())?;
            } else {
                self.set_pid(process.name(), None)?;
                self.set_state(process.name(), ProcessState::Stopped)?;
            }
        }

        if hard || self.needs_to_refresh_binaries()? {
            self.refresh_binaries(hard).await?;
            self.set_binaries_updated_at(Utc::now())?;
        }
        if hard || self.needs_to_refresh_config()? {
            self.refresh_processes()?;
            self.refresh_stacks()?;
            self.set_config_updated_at(Utc::now())?;
        }

        Ok(())
    }

    fn needs_to_refresh_binaries(&self) -> Result<bool> {
        let elapsed_since_last_update = self.get_elapsed_since_last_binaries_update()?;
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
        let elapsed_since_last_update = self.get_elapsed_since_last_config_update()?;
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

    async fn fetch_bins(target_dir: &Path) -> Result<Vec<BinaryPackage>> {
        Ok(Cargo::metadata(target_dir)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn refresh_binaries(&self, hard: bool) -> Result<()> {
        if !hard {
            return Ok(());
        }
        let binaries: Vec<BinaryPackage> = Self::fetch_bins(self.get_target_dir()).await?;
        self.set_binaries(&binaries)?;
        Ok(())
    }

    fn refresh_processes(&self) -> Result<()> {
        let previous_processes: HashMap<String, Process> = self
            .get_processes()?
            .into_iter()
            .map(|p| (p.name().to_string(), p))
            .collect();
        let processes: Vec<Process> =
            if let Some(jocker_config) = ConfigFile::load(self.get_target_dir())? {
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
                self.get_binaries()?
                    .into_iter()
                    .map(|b| Process::new(b.name(), b.name()))
                    .collect()
            };
        let processes: Vec<Process> = processes
            .into_iter()
            .map(|mut p| {
                if let Some(previous_process) = previous_processes.get(p.name()) {
                    p.pid = previous_process.pid;
                    p.state = previous_process.state.clone();
                };
                p
            })
            .collect();
        self.set_processes(processes)?;

        Ok(())
    }

    fn refresh_stacks(&self) -> Result<()> {
        let mut default_stack = None;
        let stacks = if let Some(jocker_config) = ConfigFile::load(self.get_target_dir())? {
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
        self.set_stacks(stacks.values().cloned().collect::<Vec<Stack>>().as_slice())?;
        self.set_default_stack(&default_stack)?;

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

    fn get_project_id(target_dir: &PathBuf) -> String {
        let mut hasher = DefaultHasher::new();
        target_dir.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn get_or_create_state_dir(target_dir: &PathBuf) -> Result<(String, String)> {
        let (project_id, project_dir) = Self::get_or_create_project_dir(target_dir)?;

        Ok((project_id, project_dir.clone()))
    }

    fn get_or_create_project_dir(target_dir: &PathBuf) -> Result<(String, String)> {
        let project_id = Self::get_project_id(target_dir);

        let home =
            env::var("HOME").map_err(|e| Error::with_context(InnerError::Env(e.to_string()))(e))?;
        let state_dir =
            env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{home}/.local/state"));

        let project_dir = format!("{state_dir}/{JOCKER}/{project_id}");
        let project_dir_path = Path::new(&project_dir);
        if !project_dir_path.exists() {
            create_dir_all(project_dir_path)
                .map_err(Error::with_context(InnerError::Filesystem))?;
        }
        Ok((project_id, project_dir))
    }
}

use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{create_dir_all, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    sync::RwLock,
};

use sha2::{Digest, Sha256};

use crate::{
    common::{Process, ProcessState, ROCKER},
    error::{Error, InnerError, Result},
    export_info::SerializedPackage,
};

const BINARIES_FILE: &str = "binaries.json";
const PROCESSES_FILE: &str = "processes.json";

pub struct State {
    _project_dir: PathBuf,
    filename_binaries: String,
    filename_processes: String,
    file_lock: RwLock<()>,
}

impl State {
    pub fn new() -> Result<Self> {
        let (project_dir, filename_binaries, filename_processes) = Self::get_or_create_state_dir()?;
        Ok(Self {
            _project_dir: project_dir,
            filename_binaries,
            filename_processes,
            file_lock: RwLock::new(()),
        })
    }

    pub fn filename_binaries(&self) -> &str {
        &self.filename_binaries
    }

    pub fn filename_processes(&self) -> &str {
        &self.filename_processes
    }

    pub fn get_binaries(&self) -> Result<Vec<SerializedPackage>> {
        self.file_lock
            .read()
            .expect("Poisoned RwLock, cannot recover");
        let file = File::open(self.filename_binaries())
            .map_err(Error::with_context(InnerError::StateIo))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
    }

    /// Filter processes list based on given process names
    ///
    /// If [`process_names`] is empty, returns all processes
    pub fn filter_processes(&self, process_names: &[String]) -> Result<Vec<Process>> {
        if process_names.is_empty() {
            return self.get_processes();
        }
        let processes: Vec<Process> = self
            .get_processes()?
            .into_iter()
            .filter(|process| process_names.contains(&process.name))
            .collect();
        if process_names.len() != processes.len() {
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
        self.file_lock
            .read()
            .expect("Poisoned RwLock, cannot recover");
        let file = File::open(self.filename_processes())
            .map_err(Error::with_context(InnerError::StateIo))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
    }

    pub fn set_status(&self, process_name: &str, status: ProcessState) -> Result<()> {
        let mut processes: HashMap<String, Process> = self
            .get_processes()?
            .into_iter()
            .map(|process| (process_name.to_string(), process))
            .collect();
        processes
            .get_mut(process_name)
            .expect(&format!(
                "Unable to find process {process_name} in state, this should not happen"
            ))
            .status = status;
        let processes: Vec<&Process> = processes.values().collect();
        self.file_lock
            .write()
            .expect("Poisoned RwLock, cannot recover");

        let file = File::create(self.filename_binaries())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &processes)
            .map_err(Error::with_context(InnerError::StateIo))?;

        Ok(())
    }

    fn get_or_create_state_dir() -> Result<(PathBuf, String, String)> {
        let pwd =
            env::var("PWD").map_err(|e| Error::with_context(InnerError::Env(e.to_string()))(e))?;

        let mut hasher = Sha256::new();
        hasher.update(pwd);
        let hashed_pwd = hex::encode(hasher.finalize());

        let home =
            env::var("HOME").map_err(|e| Error::with_context(InnerError::Env(e.to_string()))(e))?;
        let state_dir =
            env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{home}/.local/state"));

        let project_dir = format!("{state_dir}/{ROCKER}/{hashed_pwd}");
        let project_dir_path = Path::new(&project_dir);
        if !project_dir_path.exists() {
            create_dir_all(project_dir_path)
                .map_err(Error::with_context(InnerError::Filesystem))?;
        }

        Ok((
            project_dir_path.to_path_buf(),
            Self::get_or_create_state_file(&project_dir, BINARIES_FILE)?,
            Self::get_or_create_state_file(&project_dir, PROCESSES_FILE)?,
        ))
    }

    fn get_or_create_state_file(project_dir: &str, filename: &str) -> Result<String> {
        let file = format!("{project_dir}/{filename}");
        let file_path = Path::new(&file);
        if !file_path.exists() {
            let file =
                File::create_new(file_path).map_err(Error::with_context(InnerError::Filesystem))?;
            let writer = BufWriter::new(file);
            serde_json::to_writer(writer, &vec![] as &Vec<SerializedPackage>)
                .map_err(Error::with_context(InnerError::StateIo))?;
        }
        Ok(file)
    }
}

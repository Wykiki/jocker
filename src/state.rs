use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    fs::{create_dir_all, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Read, Write},
    path::Path,
    sync::RwLock,
};

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{
    common::{Process, ProcessState, ROCKER},
    error::{Error, InnerError, Result},
    export_info::SerializedPackage,
};

const BINARIES_FILE: &str = "binaries.json";
const LOGS_FILE: &str = "logs.txt";
const LOG_PROCESS_PREFIX: &str = "log_";
const LOG_PROCESS_SUFFIX: &str = ".txt";
const PROCESSES_FILE: &str = "processes.json";

pub struct State {
    _project_dir: String,
    filename_binaries: String,
    filename_logs: String,
    filename_processes: String,
    file_lock: RwLock<()>,
}

impl State {
    pub fn new() -> Result<Self> {
        let (project_dir, filename_binaries, filename_logs, filename_processes) =
            Self::get_or_create_state_dir()?;
        Ok(Self {
            _project_dir: project_dir,
            filename_binaries,
            filename_logs,
            filename_processes,
            file_lock: RwLock::new(()),
        })
    }

    pub fn filename_binaries(&self) -> &str {
        &self.filename_binaries
    }

    pub fn filename_logs(&self) -> &str {
        &self.filename_logs
    }

    pub fn filename_log_process(&self, process: &Process) -> String {
        let project_dir = &self._project_dir;
        let process_name = process.name();
        format!("{project_dir}/{LOG_PROCESS_PREFIX}{process_name}{LOG_PROCESS_SUFFIX}")
    }

    pub fn filename_processes(&self) -> &str {
        &self.filename_processes
    }

    #[allow(unused_must_use)]
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
    #[allow(unused_must_use)]
    pub fn get_processes(&self) -> Result<Vec<Process>> {
        self.file_lock
            .read()
            .expect("Poisoned RwLock, cannot recover");
        let file = File::open(self.filename_processes())
            .map_err(Error::with_context(InnerError::StateIo))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
    }

    #[allow(unused_must_use)]
    pub fn set_status(&self, process_name: &str, status: ProcessState) -> Result<()> {
        self.file_lock
            .write()
            .expect("Poisoned RwLock, cannot recover");
        let mut processes: HashMap<String, Process> = self
            .get_processes()?
            .into_iter()
            .map(|process| (process_name.to_string(), process))
            .collect();
        if let Some(process) = processes.get_mut(process_name) {
            process.status = status;
        } else {
            return Err(Error::new(InnerError::StateIo));
        }
        let processes: Vec<&Process> = processes.values().collect();

        let file = File::create(self.filename_processes())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &processes)
            .map_err(Error::with_context(InnerError::StateIo))?;

        Ok(())
    }

    #[allow(unused_must_use)]
    pub fn set_pid(&self, process_name: &str, pid: i32) -> Result<()> {
        dbg!(pid);
        self.file_lock
            .write()
            .expect("Poisoned RwLock, cannot recover");
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
            .pid = Some(pid);
        dbg!("1");
        let processes: Vec<&Process> = processes.values().collect();
        // dbg!(&processes);

        dbg!(&processes);
        let file = File::create(self.filename_processes())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &processes)
            .map_err(Error::with_context(InnerError::StateIo))?;
        dbg!("3");

        Ok(())
    }

    #[allow(unused_must_use)]
    pub fn log<T>(&self, content: T) -> Result<()>
    where
        T: Display,
    {
        self.file_lock
            .write()
            .expect("Poisoned RwLock, cannot recover");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open(self.filename_logs())
            .map_err(Error::with_context(InnerError::Filesystem))?;
        writeln!(file, "{} : {content}", Utc::now().to_rfc3339())?;
        Ok(())
    }

    #[allow(unused_must_use)]
    pub fn log_process<T>(&self, process: &Process, content: T) -> Result<()>
    where
        T: Read,
    {
        self.file_lock
            .write()
            .expect("Poisoned RwLock, cannot recover");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .truncate(false)
            .open(self.filename_log_process(process))
            .map_err(Error::with_context(InnerError::Filesystem))?;
        let mut buf = BufReader::new(content);
        loop {
            self.log("Log loop")?;
            let bytes = match buf.fill_buf() {
                Ok(buf) => {
                    file.write_all(buf).expect("Couldn't write");

                    buf.len()
                }
                other => panic!("Some better error handling here... {:?}", other),
            };

            if bytes == 0 {
                // Seems less-than-ideal; should be some way of
                // telling if the child has actually exited vs just
                // not outputting anything.
                break;
            }
            buf.consume(bytes);
        }
        Ok(())
    }

    fn get_or_create_state_dir() -> Result<(String, String, String, String)> {
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
            project_dir.clone(),
            Self::get_or_create_state_file(&project_dir, BINARIES_FILE)?,
            Self::get_or_create_state_file(&project_dir, LOGS_FILE)?,
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

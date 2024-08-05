use std::{
    env,
    fs::{create_dir_all, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    common::{Process, ROCKER},
    error::{Error, InnerError, Result},
    export_info::SerializedPackage,
};

const BINARIES_FILE: &str = "binaries.json";
const PROCESSES_FILE: &str = "processes.json";

pub struct State {
    _project_dir: PathBuf,
    filename_binaries: String,
    filename_processes: String,
}

impl State {
    pub fn new() -> Result<Self> {
        let (project_dir, filename_binaries, filename_processes) = Self::get_or_create_state_dir()?;
        Ok(Self {
            _project_dir: project_dir,
            filename_binaries,
            filename_processes,
        })
    }

    pub fn filename_binaries(&self) -> &str {
        &self.filename_binaries
    }

    pub fn filename_processes(&self) -> &str {
        &self.filename_processes
    }

    pub fn get_binaries(&self) -> Result<Vec<SerializedPackage>> {
        let file = File::open(self.filename_binaries())
            .map_err(Error::with_context(InnerError::StateIo))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
    }

    pub fn get_processes(&self) -> Result<Vec<Process>> {
        let file = File::open(self.filename_processes())
            .map_err(Error::with_context(InnerError::StateIo))?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
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

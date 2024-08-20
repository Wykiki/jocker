use std::{
    collections::HashSet,
    env,
    fmt::Display,
    fs::{create_dir_all, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    sync::{Arc, Mutex, RwLock},
};

use chrono::Utc;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::{
    common::{Process, ProcessSql, ProcessState, ROCKER},
    error::{Error, InnerError, Result},
    export_info::{BinaryPackage, BinaryPackageSql},
};

const DB_FILE: &str = "db.sqlite3";
const BINARIES_TABLE_NAME: &str = "binary";
const BINARIES_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE binary (
        name TEXT PRIMARY KEY,
        id TEXT NOT NULL
    )
"#;
const LOGS_FILE: &str = "logs.txt";
const LOG_PROCESS_PREFIX: &str = "log_";
const LOG_PROCESS_SUFFIX: &str = ".txt";
const PROCESSES_TABLE_NAME: &str = "process";
const PROCESSES_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE process (
        name TEXT PRIMARY KEY,
        binary TEXT NOT NULL,
        status TEXT NOT NULL,
        pid INTEGER
    )
"#;

pub struct State {
    _project_dir: String,
    // filename_binaries: String,
    filename_logs: String,
    // filename_processes: String,
    file_lock: RwLock<()>,
    db: Arc<Mutex<Connection>>,
}

impl State {
    pub fn new() -> Result<Self> {
        let (project_dir, filename_logs, db_connection) = Self::get_or_create_state_dir()?;
        Ok(Self {
            _project_dir: project_dir,
            // filename_binaries: filename_binaries.clone(),
            filename_logs,
            // filename_processes: filename_processes.clone(),
            file_lock: RwLock::new(()),
            db: Arc::new(Mutex::new(db_connection)),
            // processes_db: Arc::new(Mutex::new(Connection::open(filename_processes)?)),
        })
    }

    // pub fn filename_binaries(&self) -> &str {
    //     &self.filename_binaries
    // }

    pub fn filename_logs(&self) -> &str {
        &self.filename_logs
    }

    pub fn filename_log_process(&self, process: &Process) -> String {
        let project_dir = &self._project_dir;
        let process_name = process.name();
        format!("{project_dir}/{LOG_PROCESS_PREFIX}{process_name}{LOG_PROCESS_SUFFIX}")
    }

    // pub fn filename_processes(&self) -> &str {
    //     &self.filename_processes
    // }

    pub fn get_binaries(&self) -> Result<Vec<BinaryPackage>> {
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;
        let mut stmt = db.prepare(
            r#"
                SELECT name, id
                FROM binary
            "#,
        )?;
        let bins_iter = stmt.query_map([], |row| {
            Ok(BinaryPackageSql {
                name: row.get(0)?,
                id: row.get(1)?,
            })
        })?;
        let mut binaries = vec![];
        for bin in bins_iter {
            binaries.push(bin?.try_into()?);
        }
        Ok(binaries)
        // let file = File::open(self.filename_binaries())
        //     .map_err(Error::with_context(InnerError::StateIo))?;
        // let reader = BufReader::new(file);
        // serde_json::from_reader(reader).map_err(Error::with_context(InnerError::StateIo))
    }

    pub fn set_binaries(&self, binaries: Vec<BinaryPackage>) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;

        db.execute(
            &format!(
                r#"
                    DELETE FROM {BINARIES_TABLE_NAME}
                "#
            ),
            [],
        )?;
        for bin in binaries {
            db.execute(
                &format!(
                    r#"
                        INSERT INTO {BINARIES_TABLE_NAME} (name, id)
                        VALUES ($1, $2)
                    "#
                ),
                (bin.name, bin.id.to_string()),
            )?;
        }
        Ok(())
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
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;
        let mut stmt = db.prepare(
            r#"
                SELECT name, binary, status, pid
                FROM process
            "#,
        )?;
        let procs_iter = stmt.query_map([], |row| {
            Ok(ProcessSql {
                name: row.get(0)?,
                binary: row.get(1)?,
                status: row.get(2)?,
                pid: row.get(3)?,
            })
        })?;
        let mut processes = vec![];
        for proc in procs_iter {
            processes.push(proc?.try_into()?);
        }
        Ok(processes)
    }

    pub fn set_processes(&self, processes: Vec<Process>) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;

        db.execute(
            &format!(
                r#"
                DELETE FROM {PROCESSES_TABLE_NAME}
            "#
            ),
            [],
        )?;
        for proc in processes {
            db.execute(
                &format!(
                    r#"
                    INSERT INTO {PROCESSES_TABLE_NAME} (name, binary, status, pid)
                    VALUES ($1, $2, $3, $4)
                "#
                ),
                (proc.name, proc.binary, proc.status.to_string(), proc.pid),
            )?;
        }
        Ok(())
    }

    pub fn set_status(&self, process_name: &str, status: ProcessState) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;
        db.execute(
            r#"
                UPDATE process
                SET status = ?2
                WHERE name = ?1
            "#,
            (process_name, status.to_string().as_str()),
        )?;
        Ok(())
    }

    pub fn set_pid(&self, process_name: &str, pid: i32) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|e| Error::new(InnerError::Lock(e.to_string())))?;
        db.execute(
            r#"
                UPDATE process
                SET pid = ?2
                WHERE name = ?1
            "#,
            (process_name, pid),
        )?;
        Ok(())
    }

    pub fn log<T>(&self, content: T) -> Result<()>
    where
        T: Display,
    {
        let _lock = self
            .file_lock
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

    pub fn log_process<T>(&self, process: &Process, content: T) -> Result<()>
    where
        T: Read,
    {
        let _lock = self
            .file_lock
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

    fn get_or_create_state_dir() -> Result<(String, String, Connection)> {
        let project_dir = Self::get_or_create_project_dir()?;

        Ok((
            project_dir.clone(),
            Self::get_or_create_state_file(&project_dir, LOGS_FILE)?,
            Self::get_or_create_database(&project_dir, DB_FILE)?,
            // Self::get_or_create_database(&project_dir, PROCESSES_DB_FILE, PROCESSES_TABLE_NAME)?,
        ))
    }

    fn get_or_create_project_dir() -> Result<String> {
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
        Ok(project_dir)
    }

    fn get_or_create_state_file(project_dir: &str, filename: &str) -> Result<String> {
        let file = format!("{project_dir}/{filename}");
        let file_path = Path::new(&file);
        if !file_path.exists() {
            File::create_new(file_path).map_err(Error::with_context(InnerError::Filesystem))?;
        }
        Ok(file)
    }

    fn get_or_create_database(project_dir: &str, filename: &str) -> Result<Connection> {
        let database_file = Self::get_or_create_state_file(project_dir, filename)?;
        let conn = Connection::open(database_file)?;
        Self::init_db(&conn, BINARIES_TABLE_NAME, BINARIES_TABLE_INIT_SQL)?;
        Self::init_db(&conn, PROCESSES_TABLE_NAME, PROCESSES_TABLE_INIT_SQL)?;
        Ok(conn)
    }

    fn init_db(conn: &Connection, table_name: &str, init_query: &str) -> Result<()> {
        let table_exists = conn.query_row(
            r#"
                SELECT COUNT(name) FROM sqlite_master WHERE type='table' AND name = $1;
            "#,
            [table_name],
            |row| row.get(0).map(|count: i32| count == 1),
        )?;
        if !table_exists {
            conn.execute(init_query, ())?;
        }
        Ok(())
    }
}

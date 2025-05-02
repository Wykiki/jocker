use std::{
    collections::{HashMap, HashSet},
    env,
    fmt::Display,
    fs::{canonicalize, create_dir_all, File, OpenOptions},
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, RwLock},
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};

use crate::{
    command::cargo::{BinaryPackage, BinaryPackageSql, Cargo},
    common::{
        ConfigFile, ConfigStack, Process, ProcessSql, ProcessState, Stack, JOCKER,
        MAX_RECURSION_LEVEL,
    },
    database::Database,
    error::{lock_error, Error, InnerError, Result},
};

const DB_FILE: &str = "db.sqlite3";
const METADATA_TABLE_NAME: &str = "metadata";
const METADATA_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE metadata (
        id                  BIGINT PRIMARY KEY,
        binaries_updated_at DATETIME,
        config_updated_at   DATETIME,
        default_stack       TEXT REFERENCES stack(name) ON DELETE SET NULL
    )
"#;
const BINARY_TABLE_NAME: &str = "binary";
const BINARY_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE binary (
        name  TEXT PRIMARY KEY,
        id    TEXT NOT NULL
    )
"#;
const LOGS_FILE: &str = "logs.txt";
const LOG_PROCESS_PREFIX: &str = "log_";
const LOG_PROCESS_SUFFIX: &str = ".txt";
const PROCESS_TABLE_NAME: &str = "process";
const PROCESS_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE process (
        name        TEXT PRIMARY KEY,
        binary      TEXT NOT NULL,
        status      TEXT NOT NULL,
        pid         INTEGER,
        args        JSONB,
        cargo_args  JSONB,
        env         JSONB
    )
"#;
const STACK_TABLE_NAME: &str = "stack";
const STACK_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE stack (
        name    TEXT PRIMARY KEY
    )
"#;
const REL_STACK_PROCESS_TABLE_NAME: &str = "rel_stack_process";
const REL_STACK_PROCESS_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE rel_stack_process (
        stack_name    TEXT REFERENCES stack(name) ON DELETE CASCADE,
        process_name  TEXT REFERENCES process(name) ON DELETE CASCADE
    );
    CREATE INDEX idx_stack_name   ON rel_stack_process (stack_name);
    CREATE INDEX idx_process_name ON rel_stack_process (process_name);
"#;

#[derive(Debug, PartialEq)]
pub struct StateArgs {
    pub refresh: bool,
    pub stack: Option<String>,
}

pub struct State {
    project_dir: String,
    target_dir: PathBuf,
    filename_logs: String,
    file_lock: RwLock<()>,
    db: Database,
    current_stack: Arc<Mutex<Option<String>>>,
}

impl State {
    pub async fn new(
        refresh: bool,
        stack: Option<String>,
        target_dir: Option<impl Into<PathBuf>>,
    ) -> Result<Self> {
        let target_dir = target_dir.map(Into::into).unwrap_or(canonicalize(".")?);
        dbg!(1);
        let (project_dir, filename_logs, db_connection) =
            Self::get_or_create_state_dir(&target_dir)?;
        dbg!(2);
        let state = Self {
            project_dir,
            target_dir,
            filename_logs,
            file_lock: RwLock::new(()),
            db: Arc::new(Mutex::new(db_connection)),
            current_stack: Arc::new(Mutex::new(None)),
        };
        dbg!(3);
        state.refresh(refresh).await?;
        dbg!(4);
        state.set_current_stack(&stack)?;
        dbg!(5);
        Ok(state)
    }

    pub fn filename_logs(&self) -> &str {
        &self.filename_logs
    }

    pub fn filename_log_process(&self, process: &Process) -> String {
        let project_dir = &self.project_dir;
        let process_name = process.name();
        format!("{project_dir}/{LOG_PROCESS_PREFIX}{process_name}{LOG_PROCESS_SUFFIX}")
    }

    pub fn get_elapsed_since_last_binaries_update(&self) -> Result<u64> {
        let db = self.db.lock().map_err(lock_error)?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT binaries_updated_at
                FROM {METADATA_TABLE_NAME}
                LIMIT 1
            "#
        ))?;
        let date = if let Some(date) = stmt
            .query_row([], |row| row.get::<usize, Option<DateTime<Utc>>>(0))
            .optional()?
            .flatten()
        {
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
        let db = self.db.lock().map_err(lock_error)?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT config_updated_at
                FROM {METADATA_TABLE_NAME}
                LIMIT 1
            "#
        ))?;
        let date = if let Some(date) = stmt
            .query_row([], |row| row.get::<usize, Option<DateTime<Utc>>>(0))
            .optional()?
            .flatten()
        {
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
        let db = self.db.lock().map_err(lock_error)?;
        db.execute(
            &format!(
                r#"
                    INSERT INTO {METADATA_TABLE_NAME} (id, binaries_updated_at)
                    VALUES ($1, $2)
                    ON CONFLICT(id)
                    DO UPDATE SET
                        binaries_updated_at = excluded.binaries_updated_at
                "#,
            ),
            (0, date),
        )?;
        Ok(())
    }

    pub fn set_config_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;
        db.execute(
            &format!(
                r#"
                    INSERT INTO {METADATA_TABLE_NAME} (id, config_updated_at)
                    VALUES ($1, $2)
                    ON CONFLICT(id)
                    DO UPDATE SET
                        config_updated_at = excluded.config_updated_at
                "#,
            ),
            (0, date),
        )?;
        Ok(())
    }

    pub fn get_target_dir(&self) -> &Path {
        &self.target_dir
    }

    pub fn get_binaries(&self) -> Result<Vec<BinaryPackage>> {
        let db = self.db.lock().map_err(lock_error)?;
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
    }

    pub fn set_binaries(&self, binaries: Vec<BinaryPackage>) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;

        db.execute(
            &format!(
                r#"
                    DELETE FROM {BINARY_TABLE_NAME}
                "#
            ),
            [],
        )?;
        for bin in binaries {
            db.execute(
                &format!(
                    r#"
                        INSERT INTO {BINARY_TABLE_NAME} (name, id)
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
        let current_stack = self.get_current_stack()?;
        let expected_processes: Vec<String> = if !process_names.is_empty() {
            process_names.to_owned()
        } else if let Some(stack) = current_stack {
            self.get_stack(&stack)?.processes.into_iter().collect()
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
        let db = self.db.lock().map_err(lock_error)?;
        let mut stmt = db.prepare(
            r#"
                SELECT name, binary, status, pid, args, cargo_args, env
                FROM process
            "#,
        )?;
        let procs_iter = stmt.query_map([], |row| {
            Ok(ProcessSql {
                name: row.get(0)?,
                binary: row.get(1)?,
                status: row.get(2)?,
                pid: row.get(3)?,
                args: row.get(4)?,
                cargo_args: row.get(5)?,
                env: row.get(6)?,
            })
        })?;
        let mut processes = vec![];
        for proc in procs_iter {
            processes.push(proc?.try_into()?);
        }
        Ok(processes)
    }

    pub fn set_processes(&self, processes: Vec<Process>) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;

        db.execute(
            &format!(
                r#"
                    DELETE FROM {PROCESS_TABLE_NAME}
                "#
            ),
            [],
        )?;
        for proc in processes {
            db.execute(
                &format!(
                    r#"
                        INSERT INTO {PROCESS_TABLE_NAME} (name, binary, status, pid, args, cargo_args, env)
                        VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#
                ),
                (
                    proc.name,
                    proc.binary,
                    proc.status.to_string(),
                    proc.pid,
                    serde_json::to_value(proc.args)?,
                    serde_json::to_value(proc.cargo_args)?,
                    serde_json::to_value(proc.env)?,
                ),
            )?;
        }
        Ok(())
    }

    pub fn set_status(&self, process_name: &str, status: ProcessState) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;
        db.execute(
            &format!(
                r#"
                    UPDATE {PROCESS_TABLE_NAME}
                    SET status = ?2
                    WHERE name = ?1
                "#
            ),
            (process_name, status.to_string().as_str()),
        )?;
        Ok(())
    }

    pub fn set_pid(&self, process_name: &str, pid: Option<i32>) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;
        db.execute(
            &format!(
                r#"
                    UPDATE {PROCESS_TABLE_NAME}
                    SET pid = ?2
                    WHERE name = ?1
                "#
            ),
            (process_name, pid),
        )?;
        Ok(())
    }

    pub fn get_current_stack(&self) -> Result<Option<String>> {
        Ok(self.current_stack.lock().map_err(lock_error)?.clone())
    }

    pub fn get_default_stack(&self) -> Result<Option<String>> {
        let db = self.db.lock().map_err(lock_error)?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT default_stack
                FROM {METADATA_TABLE_NAME}
                WHERE id = $1
                LIMIT 1
            "#,
        ))?;
        let mut stmt2 = db.prepare("select * from metadata")?;
        let mut rows = stmt2.raw_query();
        while let Some(row) = rows.next().unwrap() {
            dbg!(row);
        }
        Ok(stmt.query_row([0], |row| row.get::<usize, Option<String>>(0))?)
    }

    pub fn get_stack(&self, stack: &str) -> Result<Stack> {
        let db = self.db.lock().map_err(lock_error)?;
        let name = db
            .prepare(&format!(
                r#"
                    SELECT name
                    FROM {STACK_TABLE_NAME}
                    WHERE name = $1
                    LIMIT 1
                "#,
            ))?
            .query_row([stack], |row| row.get::<usize, String>(0))
            .optional()?
            .ok_or_else(|| Error::new(InnerError::StackNotFound(stack.to_owned())))?;
        let processes: HashSet<String> = db
            .prepare(&format!(
                r#"
                    SELECT process_name
                    FROM {REL_STACK_PROCESS_TABLE_NAME}
                    WHERE stack_name = $1
                "#,
            ))?
            .query_map([stack], |row| row.get::<usize, String>(0))?
            .flat_map(std::result::Result::ok)
            .collect();
        Ok(Stack {
            name,
            processes,
            inherited_processes: Default::default(),
        })
    }

    pub fn set_current_stack(&self, stack: &Option<String>) -> Result<()> {
        if let Some(stack) = stack {
            *self.current_stack.lock().map_err(lock_error)? = Some(self.get_stack(stack)?.name);
        } else {
            dbg!(11);
            *self.current_stack.lock().map_err(lock_error)? = self.get_default_stack()?;
            dbg!(12);
        };

        Ok(())
    }

    pub fn set_default_stack(&self, stack: &Option<String>) -> Result<()> {
        let db = self.db.lock().map_err(lock_error)?;
        dbg!(31);
        db.execute(
            &format!(
                r#"
                    INSERT INTO {METADATA_TABLE_NAME} (id, default_stack)
                    VALUES ($1, $2)
                    ON CONFLICT(id)
                    DO UPDATE SET
                        default_stack = excluded.default_stack
                "#,
            ),
            (0, stack),
        )?;
        dbg!(32);
        Ok(())
    }

    pub fn set_stacks(&self, stacks: Vec<Stack>) -> Result<()> {
        let processes: HashSet<String> = self
            .get_processes()?
            .iter()
            .map(|p| p.name.to_owned())
            .collect();

        // Lock after getting processes to avoid deadlock
        let db = self.db.lock().map_err(lock_error)?;

        db.execute(
            &format!(
                r#"
                    DELETE FROM {STACK_TABLE_NAME}
                "#
            ),
            [],
        )?;
        for stack in stacks {
            let stack_processes = stack
                .processes
                .iter()
                .chain(stack.inherited_processes.iter());
            let missing_processes: Vec<String> = stack_processes
                .clone()
                .filter(|&stack_process| !processes.contains(stack_process))
                .cloned()
                .collect();
            if !missing_processes.is_empty() {
                return Err(Error::new(InnerError::ProcessNotFound(missing_processes)));
            }
            db.execute(
                &format!(
                    r#"
                        INSERT INTO {STACK_TABLE_NAME} (name)
                        VALUES ($1)
                    "#
                ),
                [&stack.name],
            )?;
            for process in stack_processes {
                db.execute(
                    &format!(
                        r#"
                        INSERT INTO {REL_STACK_PROCESS_TABLE_NAME} (stack_name, process_name)
                        VALUES ($1, $2)
                    "#
                    ),
                    (&stack.name, process),
                )?;
            }
        }
        Ok(())
    }

    // Refresh

    pub async fn refresh(&self, hard: bool) -> Result<()> {
        let user_pids = Self::get_user_pids()?;
        self.get_processes()?
            .into_iter()
            .map(|process| self.reconcile_pids(process, &user_pids))
            .collect::<Result<Vec<Process>>>()?;

        if hard || self.needs_to_refresh_binaries()? {
            self.refresh_binaries(hard).await?;
            self.set_binaries_updated_at(Utc::now())?;
        }
        if hard || self.needs_to_refresh_config()? {
            self.refresh_processes()?;
            dbg!(21);
            self.refresh_stacks()?;
            dbg!(22);
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
        self.set_binaries(binaries)?;
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
                    p.status = previous_process.status.clone();
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
        self.set_stacks(stacks.values().cloned().collect())?;
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

    fn get_or_create_state_dir(target_dir: &PathBuf) -> Result<(String, String, Connection)> {
        let project_dir = Self::get_or_create_project_dir(target_dir)?;

        Ok((
            project_dir.clone(),
            Self::get_or_create_state_file(&project_dir, LOGS_FILE)?,
            Self::get_or_create_database(&project_dir, DB_FILE)?,
        ))
    }

    fn get_or_create_project_dir(target_dir: &PathBuf) -> Result<String> {
        let pwd = target_dir;

        let mut hasher = DefaultHasher::new();
        pwd.hash(&mut hasher);
        let hashed_pwd = format!("{:x}", hasher.finish());

        let home =
            env::var("HOME").map_err(|e| Error::with_context(InnerError::Env(e.to_string()))(e))?;
        let state_dir =
            env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{home}/.local/state"));

        let project_dir = format!("{state_dir}/{JOCKER}/{hashed_pwd}");
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
        Self::init_db(&conn, METADATA_TABLE_NAME, METADATA_TABLE_INIT_SQL)?;
        Self::init_db(&conn, BINARY_TABLE_NAME, BINARY_TABLE_INIT_SQL)?;
        Self::init_db(&conn, PROCESS_TABLE_NAME, PROCESS_TABLE_INIT_SQL)?;
        Self::init_db(&conn, STACK_TABLE_NAME, STACK_TABLE_INIT_SQL)?;
        Self::init_db(
            &conn,
            REL_STACK_PROCESS_TABLE_NAME,
            REL_STACK_PROCESS_TABLE_INIT_SQL,
        )?;
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

    fn reconcile_pids(&self, mut process: Process, user_pids: &HashSet<u32>) -> Result<Process> {
        if let Some(pid) = process.pid() {
            if user_pids.get(pid).is_none() {
                self.set_status(process.name(), ProcessState::Stopped)?;
                process.status = ProcessState::Stopped;
                self.set_pid(process.name(), None)?;
                process.pid = None;
            }
        }
        Ok(process)
    }

    fn get_user_pids() -> Result<HashSet<u32>> {
        let mut run = Command::new("ps");
        let ps_output = run.arg("--no-headers").arg("o").arg("pid").output()?;
        if !ps_output.status.success() {
            return Err(Error::new(InnerError::Ps(String::from_utf8(
                ps_output.stderr,
            )?)));
        }
        Ok(String::from_utf8(ps_output.stdout)?
            .lines()
            .flat_map(|line| line.trim().parse())
            .collect())
    }
}

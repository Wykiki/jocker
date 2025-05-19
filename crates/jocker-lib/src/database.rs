use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension as _};

use crate::{
    command::cargo::{BinaryPackage, BinaryPackageSql},
    common::{Process, ProcessSql, ProcessState, Stack},
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
const PROCESS_TABLE_NAME: &str = "process";
const PROCESS_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE process (
        name        TEXT PRIMARY KEY,
        binary      TEXT NOT NULL,
        state       TEXT NOT NULL,
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
const REL_STACK_INHERITED_PROCESS_TABLE_NAME: &str = "rel_stack_inherited_process";
const REL_STACK_INHERITED_PROCESS_TABLE_INIT_SQL: &str = r#"
    CREATE TABLE rel_stack_inherited_process (
        stack_name    TEXT REFERENCES stack(name) ON DELETE CASCADE,
        process_name  TEXT REFERENCES process(name) ON DELETE CASCADE
    );
    CREATE INDEX idx_stack_name   ON rel_stack_process (stack_name);
    CREATE INDEX idx_process_name ON rel_stack_process (process_name);
"#;

pub(crate) struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub(crate) fn new(database_directory_path: impl AsRef<Path>) -> Result<Self> {
        let connection = Arc::new(Mutex::new(Self::init(database_directory_path)?));
        Ok(Self { connection })
    }

    pub(crate) fn get_binaries(&self) -> Result<Vec<BinaryPackageSql>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(
            r#"
                SELECT name, id
                FROM binary
                ORDER BY name
            "#,
        )?;

        let ret: Vec<BinaryPackageSql> = stmt
            .query_map([], |row| {
                Ok(BinaryPackageSql {
                    name: row.get(0)?,
                    id: row.get(1)?,
                })
            })?
            .flat_map(std::result::Result::ok)
            .collect();
        Ok(ret)
    }

    pub(crate) fn get_binaries_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT binaries_updated_at
                FROM {METADATA_TABLE_NAME}
                LIMIT 1
            "#
        ))?;
        Ok(stmt
            .query_row([], |row| row.get::<usize, Option<DateTime<Utc>>>(0))
            .optional()?
            .flatten())
    }

    pub(crate) fn get_config_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT config_updated_at
                FROM {METADATA_TABLE_NAME}
                LIMIT 1
            "#
        ))?;
        Ok(stmt
            .query_row([], |row| row.get::<usize, Option<DateTime<Utc>>>(0))
            .optional()?
            .flatten())
    }

    pub(crate) fn get_default_stack(&self) -> Result<Option<String>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(&format!(
            r#"
                SELECT default_stack
                FROM {METADATA_TABLE_NAME}
                WHERE id = $1
                LIMIT 1
            "#,
        ))?;
        Ok(stmt
            .query_row([0], |row| row.get::<usize, Option<String>>(0))
            .optional()?
            .flatten())
    }

    pub(crate) fn get_processes(&self) -> Result<Vec<Process>> {
        let db = self.get_db()?;
        let mut stmt = db.prepare(
            r#"
                SELECT name, binary, state, pid, args, cargo_args, env
                FROM process
                ORDER BY name ASC
            "#,
        )?;
        let procs_iter = stmt.query_map([], |row| {
            Ok(ProcessSql {
                name: row.get(0)?,
                binary: row.get(1)?,
                state: row.get(2)?,
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

    pub(crate) fn get_stack(&self, stack: &str) -> Result<Stack> {
        let db = self.get_db()?;
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
        let inherited_processes: HashSet<String> = db
            .prepare(&format!(
                r#"
                    SELECT process_name
                    FROM {REL_STACK_INHERITED_PROCESS_TABLE_NAME}
                    WHERE stack_name = $1
                "#,
            ))?
            .query_map([stack], |row| row.get::<usize, String>(0))?
            .flat_map(std::result::Result::ok)
            .collect();
        Ok(Stack {
            name,
            processes,
            inherited_processes,
        })
    }

    pub(crate) fn set_binaries(&self, binaries: &[BinaryPackage]) -> Result<()> {
        let db = self.get_db()?;

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
                (&bin.name, bin.id.to_string()),
            )?;
        }
        Ok(())
    }

    pub(crate) fn set_binaries_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let db = self.get_db()?;
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

    pub(crate) fn set_config_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let db = self.get_db()?;
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

    pub(crate) fn set_default_stack(&self, stack: &Option<String>) -> Result<()> {
        let db = self.get_db()?;
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
        Ok(())
    }

    pub(crate) fn set_process_pid(&self, process_name: &str, pid: Option<i32>) -> Result<()> {
        let db = self.get_db()?;
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

    pub(crate) fn set_process_state(&self, process_name: &str, state: ProcessState) -> Result<()> {
        let db = self.get_db()?;
        db.execute(
            &format!(
                r#"
                    UPDATE {PROCESS_TABLE_NAME}
                    SET state = ?2
                    WHERE name = ?1
                "#
            ),
            (process_name, state.to_string().as_str()),
        )?;
        Ok(())
    }

    pub(crate) fn set_processes(&self, processes: &[Process]) -> Result<()> {
        let db = self.get_db()?;

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
                        INSERT INTO {PROCESS_TABLE_NAME} (name, binary, state, pid, args, cargo_args, env)
                        VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#
                ),
                (
                    &proc.name,
                    &proc.binary,
                    proc.state.to_string(),
                    proc.pid,
                    serde_json::to_value(&proc.args)?,
                    serde_json::to_value(&proc.cargo_args)?,
                    serde_json::to_value(&proc.env)?,
                ),
            )?;
        }
        Ok(())
    }

    pub(crate) fn set_stacks(&self, stacks: &[Stack]) -> Result<()> {
        let processes: HashSet<String> = self
            .get_processes()?
            .iter()
            .map(|p| p.name.to_owned())
            .collect();

        // Lock after getting processes to avoid deadlock
        let db = self.get_db()?;

        db.execute(
            &format!(
                r#"
                    DELETE FROM {STACK_TABLE_NAME}
                "#
            ),
            [],
        )?;
        for stack in stacks {
            let stack_processes = stack.processes.iter();
            let inherited_processes = stack.inherited_processes.iter();
            let missing_processes: Vec<String> = stack_processes
                .clone()
                .chain(inherited_processes.clone())
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
            for process in inherited_processes {
                db.execute(
                    &format!(
                        r#"
                        INSERT INTO {REL_STACK_INHERITED_PROCESS_TABLE_NAME} (stack_name, process_name)
                        VALUES ($1, $2)
                    "#
                    ),
                    (&stack.name, process),
                )?;
            }
        }
        Ok(())
    }

    fn get_db(&self) -> Result<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(lock_error)
    }

    fn init(database_directory_path: impl AsRef<Path>) -> Result<Connection> {
        let database_path = database_directory_path.as_ref().join(DB_FILE);
        let conn = Connection::open(database_path)?;
        Self::init_db(&conn, METADATA_TABLE_NAME, METADATA_TABLE_INIT_SQL)?;
        Self::init_db(&conn, BINARY_TABLE_NAME, BINARY_TABLE_INIT_SQL)?;
        Self::init_db(&conn, PROCESS_TABLE_NAME, PROCESS_TABLE_INIT_SQL)?;
        Self::init_db(&conn, STACK_TABLE_NAME, STACK_TABLE_INIT_SQL)?;
        Self::init_db(
            &conn,
            REL_STACK_PROCESS_TABLE_NAME,
            REL_STACK_PROCESS_TABLE_INIT_SQL,
        )?;
        Self::init_db(
            &conn,
            REL_STACK_INHERITED_PROCESS_TABLE_NAME,
            REL_STACK_INHERITED_PROCESS_TABLE_INIT_SQL,
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
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, thread::sleep, time::Duration};

    use tempfile::{tempdir, TempDir};
    use url::Url;

    use super::*;

    #[test]
    fn get_set_binaries() {
        let (dir, db) = setup().unwrap();
        let base_url = format!("file://{}", dir.path().to_str().unwrap());
        let source_bins = [
            BinaryPackage {
                name: "foo".to_owned(),
                id: Url::parse(&format!("{base_url}/foo")).unwrap(),
            },
            BinaryPackage {
                name: "bar".to_owned(),
                id: Url::parse(&format!("{base_url}/bar")).unwrap(),
            },
            BinaryPackage {
                name: "baz".to_owned(),
                id: Url::parse(&format!("{base_url}/baz")).unwrap(),
            },
        ];

        let bins = db.get_binaries().unwrap();
        assert_eq!(bins.len(), 0);

        db.set_binaries(&source_bins[0..1]).unwrap();
        let bins = db.get_binaries().unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, source_bins[0].name);
        assert_eq!(bins[0].id, source_bins[0].id.to_string());

        db.set_binaries(&source_bins[1..2]).unwrap();
        let bins = db.get_binaries().unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, source_bins[1].name);
        assert_eq!(bins[0].id, source_bins[1].id.to_string());

        db.set_binaries(&source_bins).unwrap();
        let bins = db.get_binaries().unwrap();
        assert_eq!(bins.len(), 3);
        // Test order
        assert_eq!(bins[0].name, source_bins[1].name);
        assert_eq!(bins[0].id, source_bins[1].id.to_string());
        assert_eq!(bins[1].name, source_bins[2].name);
        assert_eq!(bins[1].id, source_bins[2].id.to_string());
        assert_eq!(bins[2].name, source_bins[0].name);
        assert_eq!(bins[2].id, source_bins[0].id.to_string());
    }

    #[test]
    fn get_set_binaries_updated_at() {
        let (dir, db) = setup().unwrap();

        let date = db.get_binaries_updated_at().unwrap();
        assert!(date.is_none());
        sleep(Duration::from_millis(100));

        let now = Utc::now();
        db.set_binaries_updated_at(now).unwrap();
        let date = db.get_binaries_updated_at().unwrap();
        assert_eq!(date, Some(now));

        drop(dir);
    }

    #[test]
    fn get_set_config_updated_at() {
        let (dir, db) = setup().unwrap();

        let date = db.get_config_updated_at().unwrap();
        assert!(date.is_none());

        let now = Utc::now();
        db.set_config_updated_at(now).unwrap();
        let date = db.get_config_updated_at().unwrap();
        assert_eq!(date, Some(now));

        drop(dir);
    }

    #[test]
    fn get_set_default_stack() {
        let (dir, db) = setup().unwrap();

        let stack = db.get_default_stack().unwrap();
        assert!(stack.is_none());

        let default_stack = None;
        db.set_default_stack(&default_stack).unwrap();
        let stack = db.get_default_stack().unwrap();
        assert_eq!(stack, default_stack);

        let default_stack = Some("foo".to_owned());
        let err = db.set_default_stack(&default_stack);
        assert!(err.is_err());

        let processes = test_processes();
        db.set_processes(&processes).unwrap();
        let stacks = test_stacks();
        db.set_stacks(&stacks).unwrap();
        let default_stack = Some("foo".to_owned());
        db.set_default_stack(&default_stack).unwrap();
        let stack = db.get_default_stack().unwrap();
        assert_eq!(stack, default_stack);

        let default_stack = None;
        db.set_default_stack(&default_stack).unwrap();
        let stack = db.get_default_stack().unwrap();
        assert_eq!(stack, default_stack);

        drop(dir);
    }

    #[test]
    fn get_set_process_properties() {
        let (dir, db) = setup().unwrap();

        let processes = db.get_processes().unwrap();
        assert!(processes.is_empty());

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).unwrap();
        db.set_process_pid(&expected_processes[0].name, Some(42))
            .unwrap();
        db.set_process_state(&expected_processes[0].name, ProcessState::Building)
            .unwrap();
        let processes = db.get_processes().unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0], expected_processes[1]);
        assert_eq!(processes[1].name, expected_processes[0].name);
        assert_eq!(processes[1].pid(), &Some(42));
        assert_eq!(processes[1].state, ProcessState::Building);

        drop(dir);
    }

    #[test]
    fn get_set_processes() {
        let (dir, db) = setup().unwrap();

        let processes = db.get_processes().unwrap();
        assert!(processes.is_empty());

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).unwrap();
        let processes = db.get_processes().unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0], expected_processes[1]);
        assert_eq!(processes[1], expected_processes[0]);

        db.set_processes(&expected_processes[1..=1]).unwrap();
        let processes = db.get_processes().unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0], expected_processes[1]);

        drop(dir);
    }

    #[test]
    fn get_set_stacks() {
        let (dir, db) = setup().unwrap();

        let stack = db.get_stack("foo").unwrap_err();
        dbg!(&stack);
        assert!(matches!(stack.inner_error, InnerError::StackNotFound(_)));

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).unwrap();
        let expected_stacks = test_stacks();
        db.set_stacks(&expected_stacks).unwrap();
        let stack = db.get_stack("foo").unwrap();
        assert_eq!(&stack.name, "foo");
        assert_eq!(stack.processes, HashSet::from(["bar".to_owned()]));
        assert_eq!(stack.inherited_processes, HashSet::new());
        let stack = db.get_stack("baz").unwrap();
        assert_eq!(&stack.name, "baz");
        assert_eq!(stack.processes, HashSet::from(["foo".to_owned()]));
        assert_eq!(stack.inherited_processes, HashSet::from(["bar".to_owned()]));

        db.set_processes(&expected_processes[1..=1]).unwrap();
        let processes = db.get_processes().unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0], expected_processes[1]);

        drop(dir);
    }

    fn setup() -> Result<(TempDir, Database)> {
        let dir = tempdir()?;
        let db = Database::new(&dir)?;
        Ok((dir, db))
    }

    fn test_processes() -> Vec<Process> {
        vec![
            Process {
                name: "foo".to_owned(),
                binary: "foo".to_owned(),
                state: ProcessState::Stopped,
                pid: None,
                args: Vec::new(),
                cargo_args: Vec::new(),
                env: HashMap::new(),
            },
            Process {
                name: "bar".to_owned(),
                binary: "bar".to_owned(),
                state: ProcessState::Stopped,
                pid: None,
                args: Vec::new(),
                cargo_args: Vec::new(),
                env: HashMap::new(),
            },
        ]
    }

    fn test_stacks() -> Vec<Stack> {
        vec![
            Stack {
                name: "foo".to_owned(),
                processes: HashSet::from(["bar".to_owned()]),
                inherited_processes: Default::default(),
            },
            Stack {
                name: "baz".to_owned(),
                processes: HashSet::from(["foo".to_owned()]),
                inherited_processes: HashSet::from(["bar".to_owned()]),
            },
        ]
    }
}

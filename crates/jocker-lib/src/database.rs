use std::{collections::HashSet, path::Path, str::FromStr as _};

use chrono::{DateTime, TimeZone, Utc};
use sqlx::{Pool, Sqlite, SqlitePool};
use tokio::fs::File;
use url::Url;

use crate::{
    command::cargo::BinaryPackage,
    common::{Process, ProcessState, Stack},
    error::{Error, InnerError, Result},
};

const DB_FILE: &str = "db.sqlite3";

pub struct BinaryPackageSql {
    pub name: String,
    pub id: String,
}

impl TryFrom<BinaryPackageSql> for BinaryPackage {
    type Error = Error;

    fn try_from(value: BinaryPackageSql) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            id: Url::from_str(&value.id)?,
        })
    }
}

pub struct ProcessSql {
    pub name: String,
    pub binary: String,
    pub state: String,
    pub pid: Option<i64>,
    pub args: String,
    pub cargo_args: String,
    pub env: String,
}

impl TryFrom<ProcessSql> for Process {
    type Error = Error;

    fn try_from(value: ProcessSql) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            binary: value.binary,
            state: value.state.try_into()?,
            pid: value.pid.map(TryFrom::try_from).transpose()?,
            args: serde_json::from_str(&value.args)?,
            cargo_args: serde_json::from_str(&value.cargo_args)?,
            env: serde_json::from_str(&value.env)?,
        })
    }
}

pub(crate) struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub(crate) async fn new(database_directory_path: impl AsRef<Path>) -> Result<Self> {
        let pool = Self::init_pool(&database_directory_path).await?;
        Ok(Self { pool })
    }

    pub(crate) async fn get_binaries(&self) -> Result<Vec<BinaryPackage>> {
        let mut conn = self.pool.acquire().await?;
        let binaries = sqlx::query_as!(
            BinaryPackageSql,
            r#"
                SELECT name, id
                FROM binary
                ORDER BY name
            "#,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<_>>()?;
        Ok(binaries)
    }

    pub(crate) async fn get_binaries_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let mut conn = self.pool.acquire().await?;
        let binaries_updated_at = sqlx::query_scalar!(
            r#"
                SELECT binaries_updated_at
                FROM metadata
                LIMIT 1
            "#,
        )
        .fetch_optional(&mut *conn)
        .await?
        .flatten()
        .map(|v| Utc.from_utc_datetime(&v));
        Ok(binaries_updated_at)
    }

    pub(crate) async fn get_config_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let mut conn = self.pool.acquire().await?;
        let config_updated_at = sqlx::query_scalar!(
            r#"
                SELECT config_updated_at
                FROM metadata
                LIMIT 1
            "#,
        )
        .fetch_optional(&mut *conn)
        .await?
        .flatten()
        .map(|v| Utc.from_utc_datetime(&v));
        Ok(config_updated_at)
    }

    pub(crate) async fn get_default_stack(&self) -> Result<Option<String>> {
        let mut conn = self.pool.acquire().await?;
        let default_stack = sqlx::query_scalar!(
            r#"
                SELECT default_stack
                FROM metadata
                LIMIT 1
            "#,
        )
        .fetch_optional(&mut *conn)
        .await?
        .flatten();
        Ok(default_stack)
    }

    pub(crate) async fn get_processes(&self) -> Result<Vec<Process>> {
        let mut conn = self.pool.acquire().await?;
        let processes = sqlx::query_as!(
            ProcessSql,
            r#"
                SELECT name, binary, state, pid, args, cargo_args, env
                FROM process
                ORDER BY name ASC
            "#,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>>>()?;
        Ok(processes)
    }

    pub(crate) async fn get_stack(&self, stack: &str) -> Result<Stack> {
        let mut conn = self.pool.begin().await?;
        let name = sqlx::query_scalar!(
            r#"
                SELECT name
                FROM stack
                WHERE name = $1
            "#,
            stack,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| Error::new(InnerError::StackNotFound(stack.to_owned())))?;
        let processes: HashSet<String> = sqlx::query_scalar!(
            r#"
                SELECT process_name
                FROM rel_stack_process
                WHERE stack_name = $1
            "#,
            stack,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
        let inherited_processes: HashSet<String> = sqlx::query_scalar!(
            r#"
                SELECT process_name
                FROM rel_stack_inherited_process
                WHERE stack_name = $1
            "#,
            stack,
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
        conn.commit().await?;
        Ok(Stack {
            name,
            processes,
            inherited_processes,
        })
    }

    pub(crate) async fn set_binaries(&self, binaries: &[BinaryPackage]) -> Result<()> {
        let mut conn = self.pool.begin().await?;
        sqlx::query!(
            r#"
                DELETE FROM binary
            "#,
        )
        .execute(&mut *conn)
        .await?;
        for bin in binaries {
            let id = bin.id.to_string();
            sqlx::query!(
                r#"
                    INSERT INTO binary (name, id)
                    VALUES ($1, $2)
                "#,
                bin.name,
                id,
            )
            .execute(&mut *conn)
            .await?;
        }
        conn.commit().await?;
        Ok(())
    }

    pub(crate) async fn set_binaries_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(
            r#"
                INSERT INTO metadata (id, binaries_updated_at)
                VALUES ($1, $2)
                ON CONFLICT(id)
                DO UPDATE SET
                    binaries_updated_at = excluded.binaries_updated_at
            "#,
            0,
            date,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_config_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(
            r#"
                INSERT INTO metadata (id, config_updated_at)
                VALUES ($1, $2)
                ON CONFLICT(id)
                DO UPDATE SET
                    config_updated_at = excluded.config_updated_at
            "#,
            0,
            date,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_default_stack(&self, stack: &Option<String>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(
            r#"
                INSERT INTO metadata (id, default_stack)
                VALUES ($1, $2)
                ON CONFLICT(id)
                DO UPDATE SET
                    default_stack = excluded.default_stack
            "#,
            0,
            stack,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_process_pid(&self, process_name: &str, pid: Option<i32>) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        sqlx::query!(
            r#"
                UPDATE process
                SET pid = ?2
                WHERE name = ?1
            "#,
            process_name,
            pid,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_process_state(
        &self,
        process_name: &str,
        state: ProcessState,
    ) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        let state = state.to_string();
        sqlx::query!(
            r#"
                UPDATE process
                SET state = ?2
                WHERE name = ?1
            "#,
            process_name,
            state,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    pub(crate) async fn set_processes(&self, processes: &[Process]) -> Result<()> {
        let mut conn = self.pool.begin().await?;

        sqlx::query!(
            r#"
                DELETE FROM process
            "#,
        )
        .execute(&mut *conn)
        .await?;
        for proc in processes {
            let state = proc.state.to_string();
            let pid: Option<i64> = proc.pid.map(TryInto::try_into).transpose()?;
            let args = serde_json::to_value(&proc.args)?;
            let cargo_args = serde_json::to_value(&proc.cargo_args)?;
            let env = serde_json::to_value(&proc.env)?;
            sqlx::query!(
                r#"
                    INSERT INTO process (name, binary, state, pid, args, cargo_args, env)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                proc.name,
                proc.binary,
                state,
                pid,
                args,
                cargo_args,
                env,
            )
            .execute(&mut *conn)
            .await?;
        }
        conn.commit().await?;
        Ok(())
    }

    pub(crate) async fn set_stacks(&self, stacks: &[Stack]) -> Result<()> {
        let processes: HashSet<String> = self
            .get_processes()
            .await?
            .iter()
            .map(|p| p.name.to_owned())
            .collect();

        // Lock after getting processes to avoid deadlock
        let mut conn = self.pool.begin().await?;

        sqlx::query!(
            r#"
                DELETE FROM stack
            "#,
        )
        .execute(&mut *conn)
        .await?;
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
            sqlx::query!(
                r#"
                    INSERT INTO stack (name)
                    VALUES ($1)
                "#,
                stack.name,
            )
            .execute(&mut *conn)
            .await?;
            for process in stack_processes {
                sqlx::query!(
                    r#"
                        INSERT INTO rel_stack_process (stack_name, process_name)
                        VALUES ($1, $2)
                    "#,
                    stack.name,
                    process,
                )
                .execute(&mut *conn)
                .await?;
            }
            for process in inherited_processes {
                sqlx::query!(
                    r#"
                        INSERT INTO rel_stack_inherited_process (stack_name, process_name)
                        VALUES ($1, $2)
                    "#,
                    stack.name,
                    process,
                )
                .execute(&mut *conn)
                .await?;
            }
        }

        conn.commit().await?;
        Ok(())
    }

    async fn init_pool(database_directory_path: impl AsRef<Path>) -> Result<Pool<Sqlite>> {
        let database_path = database_directory_path.as_ref().join(DB_FILE);
        if !database_path.exists() {
            File::create(&database_path).await?;
        }

        let pool = SqlitePool::connect(
            database_path
                .to_str()
                .ok_or_else(|| Error::new(InnerError::Filesystem))?,
        )
        .await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(pool)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, thread::sleep, time::Duration};

    use tempfile::{tempdir, TempDir};
    use url::Url;

    use super::*;

    #[tokio::test]
    async fn get_set_binaries() {
        let (dir, db) = setup().await.unwrap();
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

        let bins = db.get_binaries().await.unwrap();
        assert_eq!(bins.len(), 0);

        db.set_binaries(&source_bins[0..1]).await.unwrap();
        let bins = db.get_binaries().await.unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, source_bins[0].name);
        assert_eq!(bins[0].id, source_bins[0].id);

        db.set_binaries(&source_bins[1..2]).await.unwrap();
        let bins = db.get_binaries().await.unwrap();
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, source_bins[1].name);
        assert_eq!(bins[0].id, source_bins[1].id);

        db.set_binaries(&source_bins).await.unwrap();
        let bins = db.get_binaries().await.unwrap();
        assert_eq!(bins.len(), 3);
        // Test order
        assert_eq!(bins[0].name, source_bins[1].name);
        assert_eq!(bins[0].id, source_bins[1].id);
        assert_eq!(bins[1].name, source_bins[2].name);
        assert_eq!(bins[1].id, source_bins[2].id);
        assert_eq!(bins[2].name, source_bins[0].name);
        assert_eq!(bins[2].id, source_bins[0].id);
    }

    #[tokio::test]
    async fn get_set_binaries_updated_at() {
        let (dir, db) = setup().await.unwrap();

        let date = db.get_binaries_updated_at().await.unwrap();
        assert!(date.is_none());
        sleep(Duration::from_millis(100));

        let now = Utc::now();
        db.set_binaries_updated_at(now).await.unwrap();
        let date = db.get_binaries_updated_at().await.unwrap();
        assert_eq!(date, Some(now));

        drop(dir);
    }

    #[tokio::test]
    async fn get_set_config_updated_at() {
        let (dir, db) = setup().await.unwrap();

        let date = db.get_config_updated_at().await.unwrap();
        assert!(date.is_none());

        let now = Utc::now();
        db.set_config_updated_at(now).await.unwrap();
        let date = db.get_config_updated_at().await.unwrap();
        assert_eq!(date, Some(now));

        drop(dir);
    }

    #[tokio::test]
    async fn get_set_default_stack() {
        let (dir, db) = setup().await.unwrap();

        let stack = db.get_default_stack().await.unwrap();
        assert!(stack.is_none());

        let default_stack = None;
        db.set_default_stack(&default_stack).await.unwrap();
        let stack = db.get_default_stack().await.unwrap();
        assert_eq!(stack, default_stack);

        let default_stack = Some("foo".to_owned());
        let err = db.set_default_stack(&default_stack).await;
        assert!(err.is_err());

        let processes = test_processes();
        db.set_processes(&processes).await.unwrap();
        let stacks = test_stacks();
        db.set_stacks(&stacks).await.unwrap();
        let default_stack = Some("foo".to_owned());
        db.set_default_stack(&default_stack).await.unwrap();
        let stack = db.get_default_stack().await.unwrap();
        assert_eq!(stack, default_stack);

        let default_stack = None;
        db.set_default_stack(&default_stack).await.unwrap();
        let stack = db.get_default_stack().await.unwrap();
        assert_eq!(stack, default_stack);

        drop(dir);
    }

    #[tokio::test]
    async fn get_set_process_properties() {
        let (dir, db) = setup().await.unwrap();

        let processes = db.get_processes().await.unwrap();
        assert!(processes.is_empty());

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).await.unwrap();
        db.set_process_pid(&expected_processes[0].name, Some(42))
            .await
            .unwrap();
        db.set_process_state(&expected_processes[0].name, ProcessState::Building)
            .await
            .unwrap();
        let processes = db.get_processes().await.unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0], expected_processes[1]);
        assert_eq!(processes[1].name, expected_processes[0].name);
        assert_eq!(processes[1].pid(), &Some(42));
        assert_eq!(processes[1].state, ProcessState::Building);

        drop(dir);
    }

    #[tokio::test]
    async fn get_set_processes() {
        let (dir, db) = setup().await.unwrap();

        let processes = db.get_processes().await.unwrap();
        assert!(processes.is_empty());

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).await.unwrap();
        let processes = db.get_processes().await.unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0], expected_processes[1]);
        assert_eq!(processes[1], expected_processes[0]);

        db.set_processes(&expected_processes[1..=1]).await.unwrap();
        let processes = db.get_processes().await.unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0], expected_processes[1]);

        drop(dir);
    }

    #[tokio::test]
    async fn get_set_stacks() {
        let (dir, db) = setup().await.unwrap();

        let stack = db.get_stack("foo").await.unwrap_err();
        assert!(matches!(stack.inner_error, InnerError::StackNotFound(_)));

        let expected_processes = test_processes();
        db.set_processes(&expected_processes).await.unwrap();
        let expected_stacks = test_stacks();
        db.set_stacks(&expected_stacks).await.unwrap();
        let stack = db.get_stack("foo").await.unwrap();
        assert_eq!(&stack.name, "foo");
        assert_eq!(stack.processes, HashSet::from(["bar".to_owned()]));
        assert_eq!(stack.inherited_processes, HashSet::new());
        let stack = db.get_stack("baz").await.unwrap();
        assert_eq!(&stack.name, "baz");
        assert_eq!(stack.processes, HashSet::from(["foo".to_owned()]));
        assert_eq!(stack.inherited_processes, HashSet::from(["bar".to_owned()]));

        db.set_processes(&expected_processes[1..=1]).await.unwrap();
        let processes = db.get_processes().await.unwrap();
        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0], expected_processes[1]);

        drop(dir);
    }

    async fn setup() -> Result<(TempDir, Database)> {
        let dir = tempdir()?;
        let db = Database::new(&dir).await?;
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

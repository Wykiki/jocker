use std::{collections::HashSet, path::Path, sync::Arc};

use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable, TableDefinition, TypeName, Value};
use serde::{Deserialize, Serialize};

use crate::{
    common::{Binary, Process, ProcessState, Stack},
    error::{Error, InnerError, Result},
};

const DB_FILE: &str = "db.redb";
const METADATA: TableDefinition<u8, Metadata> = TableDefinition::new("metadata");
const BINARY: TableDefinition<&str, Binary> = TableDefinition::new("binary");
const PROCESS: TableDefinition<&str, Process> = TableDefinition::new("process");
const STACK: TableDefinition<&str, Stack> = TableDefinition::new("stack");

#[derive(Debug, Default, Deserialize, Serialize)]
struct Metadata {
    binaries_updated_at: DateTime<Utc>,
    config_updated_at: DateTime<Utc>,
    default_stack: Option<String>,
}

impl Value for Metadata {
    type SelfType<'a>
        = Metadata
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap()
    }

    fn type_name() -> redb::TypeName {
        TypeName::new("jocker-lib_metadata")
    }
}

impl Value for Binary {
    type SelfType<'a>
        = Binary
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new("jocker-lib_binary-package")
    }
}

impl Value for Process {
    type SelfType<'a>
        = Process
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new("jocker-lib_process")
    }
}

impl Value for Stack {
    type SelfType<'a>
        = Stack
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        serde_json::from_slice(data).unwrap()
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        serde_json::to_vec(value).unwrap()
    }

    fn type_name() -> TypeName {
        TypeName::new("jocker-lib_stack")
    }
}

pub(crate) struct Database {
    db: Arc<redb::Database>,
}

impl Database {
    pub(crate) async fn new(database_directory_path: impl AsRef<Path>) -> Result<Self> {
        let database_path = database_directory_path.as_ref().join(DB_FILE);
        let db = redb::Database::create(database_path)?;
        let txn = db.begin_write()?;
        {
            txn.open_table(METADATA)?;
            txn.open_table(BINARY)?;
            txn.open_table(PROCESS)?;
            txn.open_table(STACK)?;
        }
        txn.commit()?;
        Ok(Self { db: Arc::new(db) })
    }

    pub(crate) async fn get_binaries(&self) -> Result<Vec<Binary>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(BINARY)?;

        Ok(table
            .iter()?
            .map(|v| v.map(|vv| vv.1.value()))
            .collect::<std::result::Result<_, _>>()?)
    }

    pub(crate) async fn get_binaries_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(METADATA)?;

        Ok(table.get(0)?.map(|v| v.value().binaries_updated_at))
    }

    pub(crate) async fn get_config_updated_at(&self) -> Result<Option<DateTime<Utc>>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(METADATA)?;

        Ok(table.get(0)?.map(|v| v.value().config_updated_at))
    }

    pub(crate) async fn get_default_stack(&self) -> Result<Option<String>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(METADATA)?;

        Ok(table.get(0)?.and_then(|v| v.value().default_stack))
    }

    pub(crate) async fn get_processes(&self) -> Result<Vec<Process>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(PROCESS)?;

        Ok(table
            .iter()?
            .map(|v| v.map(|vv| vv.1.value()))
            .collect::<std::result::Result<_, _>>()?)
    }

    pub(crate) async fn get_stack(&self, stack: &str) -> Result<Stack> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(STACK)?;

        table
            .get(stack)?
            .map(|v| v.value())
            .ok_or_else(|| Error::new(InnerError::StackNotFound(stack.to_owned())))
    }

    pub(crate) async fn get_stacks(&self) -> Result<Vec<Stack>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(STACK)?;

        Ok(table
            .iter()?
            .map(|v| v.map(|vv| vv.1.value()))
            .collect::<std::result::Result<_, _>>()?)
    }

    pub(crate) async fn set_binaries(&self, binaries: &[Binary]) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            txn.delete_table(BINARY)?;
            let mut table = txn.open_table(BINARY)?;
            for bin in binaries {
                table.insert(bin.name.as_str(), bin)?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_binaries_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut metadata = txn
                .open_table(METADATA)?
                .get(0)?
                .map(|v| v.value())
                .unwrap_or_default();
            metadata.binaries_updated_at = date;
            let mut table = txn.open_table(METADATA)?;
            table.insert(0, metadata)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_config_updated_at(&self, date: DateTime<Utc>) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut metadata = txn
                .open_table(METADATA)?
                .get(0)?
                .map(|v| v.value())
                .unwrap_or_default();
            metadata.config_updated_at = date;
            let mut table = txn.open_table(METADATA)?;
            table.insert(0, metadata)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_default_stack(&self, stack: &Option<String>) -> Result<()> {
        if let Some(stack) = stack {
            self.get_stack(stack).await?;
        }
        let txn = self.db.begin_write()?;
        {
            let mut metadata = txn
                .open_table(METADATA)?
                .get(0)?
                .map(|v| v.value())
                .unwrap_or_default();
            metadata.default_stack = stack.clone();
            let mut table = txn.open_table(METADATA)?;
            table.insert(0, metadata)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_process_pid(
        &self,
        process_name: &str,
        pid: Option<usize>,
    ) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut process = txn
                .open_table(PROCESS)?
                .get(process_name)?
                .map(|v| v.value())
                .unwrap_or_default();
            process.pid = pid;
            let mut table = txn.open_table(PROCESS)?;
            table.insert(process_name, process)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_process_state(
        &self,
        process_name: &str,
        state: ProcessState,
    ) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut process = txn
                .open_table(PROCESS)?
                .get(process_name)?
                .map(|v| v.value())
                .unwrap_or_default();
            process.state = state;
            let mut table = txn.open_table(PROCESS)?;
            table.insert(process_name, process)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub(crate) async fn set_processes(&self, processes: &[Process]) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            txn.delete_table(PROCESS)?;
            let mut table = txn.open_table(PROCESS)?;
            for process in processes {
                table.insert(process.name.as_str(), process)?;
            }
        }
        txn.commit()?;
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
        let txn = self.db.begin_write()?;
        {
            txn.delete_table(STACK)?;
            let mut table = txn.open_table(STACK)?;
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
                table.insert(stack.name.as_str(), stack)?;
            }
        }
        txn.commit()?;
        Ok(())
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
            Binary {
                name: "foo".to_owned(),
                id: Url::parse(&format!("{base_url}/foo")).unwrap(),
            },
            Binary {
                name: "bar".to_owned(),
                id: Url::parse(&format!("{base_url}/bar")).unwrap(),
            },
            Binary {
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

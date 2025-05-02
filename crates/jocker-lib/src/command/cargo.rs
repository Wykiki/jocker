use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::Display,
    hash::Hash,
    path::Path,
    process::Stdio,
    str::FromStr,
};

use dotenvy::dotenv_iter;
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};
use url::Url;

use crate::error::{Error, InnerError, Result};

pub struct Cargo;

impl Cargo {
    /// Start a `cargo` subprocess that builds given binaries. Returns a handle to it.
    pub async fn build<S>(target_dir: &Path, binaries: &[S], cargo_args: &[S]) -> Result<Child>
    where
        S: AsRef<OsStr> + Display + Eq + Hash,
    {
        let mut env: HashMap<String, String> = HashMap::new();
        if let Ok(dotenv) = dotenv_iter() {
            for (key, val) in dotenv.flatten() {
                env.insert(key, val);
            }
        }
        let env = env;

        let mut build = Command::new("cargo");
        build.stdout(Stdio::piped()).stderr(Stdio::piped());
        build.arg("build");
        for arg in HashSet::<&S>::from_iter(cargo_args) {
            build.arg(arg);
        }
        for binary in HashSet::<&S>::from_iter(binaries) {
            build.arg(format!("--bin={binary}"));
        }
        for (key, val) in env.iter() {
            build.env(key, val);
        }
        build.current_dir(target_dir);
        let build = build
            .spawn()
            .map_err(Error::with_context(InnerError::Start(
                "Unable to start `cargo build` command".to_string(),
            )))?;
        Ok(build)
    }

    pub async fn metadata(target_dir: &Path) -> Result<Vec<SerializedPackage>> {
        let metadata = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .current_dir(target_dir)
            .output()
            .await
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
}

#[derive(Debug, Deserialize)]
pub struct ExportInfoMinimal {
    pub packages: Vec<SerializedPackage>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SerializedPackage {
    pub name: String,
    pub id: Url,
    pub targets: Vec<TargetInner>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct TargetInner {
    pub kind: Vec<TargetKind>,
    pub name: String,
    pub bin_name: Option<String>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetKind {
    Lib,
    Bin,
    Test,
    Bench,
    ExampleLib,
    ExampleBin,
    CustomBuild,
    #[serde(untagged)]
    Other(String),
}

pub struct BinaryPackage {
    pub name: String,
    pub id: Url,
}

impl BinaryPackage {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<SerializedPackage> for BinaryPackage {
    fn from(value: SerializedPackage) -> Self {
        Self {
            name: value.name,
            id: value.id,
        }
    }
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

pub struct BinaryPackageSql {
    pub name: String,
    pub id: String,
}

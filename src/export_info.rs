use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::Error;

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

impl TryFrom<BinaryPackageSql> for BinaryPackage {
    type Error = Error;

    fn try_from(value: BinaryPackageSql) -> Result<Self, Self::Error> {
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

use serde::{Deserialize, Serialize};
use url::Url;

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

impl SerializedPackage {
    pub fn name(&self) -> &str {
        &self.name
    }
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

use std::path::PathBuf;

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

#[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Deserialize, Serialize)]
pub struct PackageIdSpec {
    name: String,
    version: Option<PartialVersion>,
    url: Option<Url>,
    kind: Option<SourceKind>,
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize, Serialize)]
pub enum Platform {
    /// A named platform, like `x86_64-apple-darwin`.
    Name(String),
    /// A cfg expression, like `cfg(windows)`.
    Cfg(CfgExpr),
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize, Serialize)]
pub enum CfgExpr {
    Not(Box<CfgExpr>),
    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
    Value(Cfg),
}

/// A cfg value.
#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize, Serialize)]
pub enum Cfg {
    /// A named cfg value, like `unix`.
    Name(String),
    /// A key/value cfg pair, like `target_os = "linux"`.
    KeyPair(String, String),
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

/// Types of the output artifact that the compiler emits.
/// Usually distributable or linkable either statically or dynamically.
///
/// See <https://doc.rust-lang.org/nightly/reference/linkage.html>.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum CrateType {
    Bin,
    Lib,
    Rlib,
    Dylib,
    Cdylib,
    Staticlib,
    ProcMacro,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum TargetSourcePath {
    Path(PathBuf),
    Metabuild,
}

#[derive(
    Default, Clone, Copy, Debug, Hash, PartialOrd, Ord, Eq, PartialEq, Deserialize, Serialize,
)]
pub enum Edition {
    /// The 2015 edition
    #[default]
    Edition2015,
    /// The 2018 edition
    Edition2018,
    /// The 2021 edition
    Edition2021,
    /// The 2024 edition
    Edition2024,
}

/// Indicates whether a target should have examples scraped from it by rustdoc.
/// Configured within Cargo.toml and only for unstable feature
/// [`-Zrustdoc-scrape-examples`][1].
///
/// [1]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#scrape-examples
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Deserialize, Serialize)]
pub enum RustdocScrapeExamples {
    Enabled,
    Disabled,
    Unset,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Deserialize, Serialize)]
pub struct PartialVersion {
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    // pub pre: Option<semver::Prerelease>,
    // pub build: Option<semver::BuildMetadata>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum SourceKind {
    /// A git repository.
    Git(GitReference),
    /// A local path.
    Path,
    /// A remote registry.
    Registry,
    /// A sparse registry.
    SparseRegistry,
    /// A local filesystem-based registry.
    LocalRegistry,
    /// A directory-based registry.
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum GitReference {
    /// From a tag.
    Tag(String),
    /// From a branch.
    Branch(String),
    /// From a specific revision. Can be a commit hash (either short or full),
    /// or a named reference like `refs/pull/493/head`.
    Rev(String),
    /// The default branch of the repository, the reference named `HEAD`.
    DefaultBranch,
}

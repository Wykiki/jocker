use core::panic;
use std::path::PathBuf;

use semver::Version;
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ExportInfo {
    pub packages: Vec<SerializedPackage>,
    // workspace_members: Vec<PackageIdSpec>,
    // workspace_default_members: Vec<PackageIdSpec>,
    // resolve: Option<MetadataResolve>,
    // target_directory: PathBuf,
    // version: u32,
    // workspace_root: PathBuf,
    // metadata: Option<toml::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SerializedPackage {
    pub name: String,
    pub version: Version,
    pub id: Url,
    // license: Option<String>,
    // license_file: Option<String>,
    // description: Option<String>,
    // source: SourceId,
    // dependencies: Vec<Dependency>,
    pub targets: Vec<TargetInner>,
    // features: BTreeMap<String, Vec<String>>,
    pub manifest_path: PathBuf,
    // metadata: Option<toml::Value>,
    // publish: Option<Vec<String>>,
    // authors: Vec<String>,
    // categories: Vec<String>,
    // keywords: Vec<String>,
    // readme: Option<String>,
    // repository: Option<String>,
    // homepage: Option<String>,
    // documentation: Option<String>,
    // edition: String,
    // links: Option<String>,
    // #[serde(skip_serializing_if = "Option::is_none")]
    // metabuild: Option<Vec<String>>,
    // default_run: Option<String>,
    // rust_version: Option<RustVersion>,
}

#[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Deserialize)]
pub struct PackageIdSpec {
    name: String,
    version: Option<PartialVersion>,
    url: Option<Url>,
    kind: Option<SourceKind>,
}

// impl PackageIdSpec {
//     pub fn parse(spec: &str) -> PackageIdSpec {
//         if spec.contains("://") {
//             if let Ok(url) = Url::parse(spec) {
//                 return PackageIdSpec::from_url(url);
//             }
//         } else if spec.contains('/') || spec.contains('\\') {
//             let abs = std::env::current_dir().unwrap_or_default().join(spec);
//             if abs.exists() {
//                 let maybe_url = Url::from_file_path(abs)
//                     .map_or_else(|_| "a file:// URL".to_string(), |url| url.to_string());
//                 panic!();
//             }
//         }
//         let mut parts = spec.splitn(2, [':', '@']);
//         let name = parts.next().unwrap();
//         let version = match parts.next() {
//             Some(version) => Some(version.parse::<PartialVersion>()?),
//             None => None,
//         };
//         PackageIdSpec {
//             name: String::from(name),
//             version,
//             url: None,
//             kind: None,
//         }
//     }
//
//     /// Tries to convert a valid `Url` to a `PackageIdSpec`.
//     fn from_url(mut url: Url) -> Result<PackageIdSpec> {
//         let mut kind = None;
//         if let Some((kind_str, scheme)) = url.scheme().split_once('+') {
//             match kind_str {
//                 "git" => {
//                     let git_ref = GitReference::from_query(url.query_pairs());
//                     url.set_query(None);
//                     kind = Some(SourceKind::Git(git_ref));
//                     url = strip_url_protocol(&url);
//                 }
//                 "registry" => {
//                     if url.query().is_some() {
//                         return Err(ErrorKind::UnexpectedQueryString(url).into());
//                     }
//                     kind = Some(SourceKind::Registry);
//                     url = strip_url_protocol(&url);
//                 }
//                 "sparse" => {
//                     if url.query().is_some() {
//                         return Err(ErrorKind::UnexpectedQueryString(url).into());
//                     }
//                     kind = Some(SourceKind::SparseRegistry);
//                     // Leave `sparse` as part of URL, see `SourceId::new`
//                     // url = strip_url_protocol(&url);
//                 }
//                 "path" => {
//                     if url.query().is_some() {
//                         return Err(ErrorKind::UnexpectedQueryString(url).into());
//                     }
//                     if scheme != "file" {
//                         return Err(ErrorKind::UnsupportedPathPlusScheme(scheme.into()).into());
//                     }
//                     kind = Some(SourceKind::Path);
//                     url = strip_url_protocol(&url);
//                 }
//                 kind => return Err(ErrorKind::UnsupportedProtocol(kind.into()).into()),
//             }
//         } else {
//             if url.query().is_some() {
//                 return Err(ErrorKind::UnexpectedQueryString(url).into());
//             }
//         }
//
//         let frag = url.fragment().map(|s| s.to_owned());
//         url.set_fragment(None);
//
//         let (name, version) = {
//             let Some(path_name) = url.path_segments().and_then(|mut p| p.next_back()) else {
//                 panic!();
//                 // return Err(ErrorKind::MissingUrlPath(url).into());
//             };
//             match frag {
//                 Some(fragment) => match fragment.split_once([':', '@']) {
//                     Some((name, part)) => {
//                         let version = part.parse::<PartialVersion>()?;
//                         (String::from(name), Some(version))
//                     }
//                     None => {
//                         if fragment.chars().next().unwrap().is_alphabetic() {
//                             (String::from(fragment.as_str()), None)
//                         } else {
//                             let version = fragment.parse::<PartialVersion>()?;
//                             (String::from(path_name), Some(version))
//                         }
//                     }
//                 },
//                 None => (String::from(path_name), None),
//             }
//         };
//         // PackageName::new(&name)?;
//         Ok(PackageIdSpec {
//             name,
//             version,
//             url: Some(url),
//             kind,
//         })
//     }
// }
//
// #[derive(Clone, Copy, Eq, PartialOrd, Ord, Deserialize)]
// pub struct PackageId {
//     inner: &'static PackageIdInner,
// }
//
// #[derive(PartialOrd, Eq, Ord, Deserialize)]
// struct PackageIdInner {
//     name: String,
//     version: semver::Version,
//     source_id: SourceId,
// }

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize)]
pub enum Platform {
    /// A named platform, like `x86_64-apple-darwin`.
    Name(String),
    /// A cfg expression, like `cfg(windows)`.
    Cfg(CfgExpr),
}

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize)]
pub enum CfgExpr {
    Not(Box<CfgExpr>),
    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
    Value(Cfg),
}

/// A cfg value.
#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Clone, Debug, Deserialize)]
pub enum Cfg {
    /// A named cfg value, like `unix`.
    Name(String),
    /// A key/value cfg pair, like `target_os = "linux"`.
    KeyPair(String, String),
}

// #[derive(Clone, Copy, Eq, Debug)]
// pub struct SourceId {
//     inner: &'static SourceIdInner,
// }
//
// #[derive(Eq, Clone, Debug)]
// struct SourceIdInner {
//     /// The source URL.
//     url: Url,
//     /// The canonical version of the above url. See [`CanonicalUrl`] to learn
//     /// why it is needed and how it normalizes a URL.
//     canonical_url: CanonicalUrl,
//     /// The source kind.
//     kind: SourceKind,
//     /// For example, the exact Git revision of the specified branch for a Git Source.
//     precise: Option<Precise>,
//     /// Name of the remote registry.
//     ///
//     /// WARNING: this is not always set when the name is not known,
//     /// e.g. registry coming from `--index` or Cargo.lock
//     registry_key: Option<KeyOf>,
// }

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct TargetInner {
    pub kind: Vec<TargetKind>,
    pub name: String,
    // Whether the name was inferred by Cargo, or explicitly given.
    // name_inferred: bool,
    // Note that `bin_name` is used for the cargo-feature `different_binary_name`
    pub bin_name: Option<String>,
    // Note that the `src_path` here is excluded from the `Hash` implementation
    // as it's absolute currently and is otherwise a little too brittle for
    // causing rebuilds. Instead the hash for the path that we send to the
    // compiler is handled elsewhere.
    // src_path: TargetSourcePath,
    // required_features: Option<Vec<String>>,
    // tested: bool,
    // benched: bool,
    // doc: bool,
    // doctest: bool,
    // harness: bool, // whether to use the test harness (--test)
    // for_host: bool,
    // proc_macro: bool,
    // edition: Edition,
    // doc_scrape_examples: RustdocScrapeExamples,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum TargetSourcePath {
    Path(PathBuf),
    Metabuild,
}

#[derive(Default, Clone, Copy, Debug, Hash, PartialOrd, Ord, Eq, PartialEq, Deserialize)]
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
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Deserialize)]
pub enum RustdocScrapeExamples {
    Enabled,
    Disabled,
    Unset,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, Deserialize)]
pub struct PartialVersion {
    pub major: u64,
    pub minor: Option<u64>,
    pub patch: Option<u64>,
    // pub pre: Option<semver::Prerelease>,
    // pub build: Option<semver::BuildMetadata>,
}

// impl std::str::FromStr for PartialVersion {
//     type Err = ();
//
//     fn from_str(value: &str) -> Result<Self, Self::Err> {
//         match semver::Version::parse(value) {
//             Ok(ver) => Ok(ver.into()),
//             Err(_) => {
//                 // HACK: Leverage `VersionReq` for partial version parsing
//                 let mut version_req = match semver::VersionReq::parse(value) {
//                     Ok(req) => req,
//                     Err(_) => panic!(),
//                     Err(_) if value.contains('+') => return Err(ErrorKind::BuildMetadata.into()),
//                     Err(_) => return Err(ErrorKind::Unexpected.into()),
//                 };
//                 if version_req.comparators.len() != 1 {
//                     return Err(ErrorKind::VersionReq.into());
//                 }
//                 let comp = version_req.comparators.pop().unwrap();
//                 if comp.op != semver::Op::Caret {
//                     return Err(ErrorKind::VersionReq.into());
//                 } else if value.starts_with('^') {
//                     // Can't distinguish between `^` present or not
//                     return Err(ErrorKind::VersionReq.into());
//                 }
//                 let pre = if comp.pre.is_empty() {
//                     None
//                 } else {
//                     Some(comp.pre)
//                 };
//                 Ok(Self {
//                     major: comp.major,
//                     minor: comp.minor,
//                     patch: comp.patch,
//                     pre,
//                     build: None,
//                 })
//             }
//         }
//     }
// }

#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
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

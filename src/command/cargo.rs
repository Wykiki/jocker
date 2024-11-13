use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt::Display,
    hash::Hash,
    process::Stdio,
};

use dotenvy::dotenv_iter;
use tokio::process::Child;

use crate::error::{Error, InnerError, Result};

pub struct Cargo;

impl Cargo {
    /// Start a `cargo` subprocess that builds given binaries. Returns a handle to it.
    pub async fn build<S>(binaries: &[S], cargo_args: &[S]) -> Result<Child>
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

        let mut build = tokio::process::Command::new("cargo");
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
        let build = build
            .spawn()
            .map_err(Error::with_context(InnerError::Start(
                "Unable to start `cargo build` command".to_string(),
            )))?;
        Ok(build)

        // let build = build.wait().await?;
        // if !build.success() {
        //     return Err(Error::new(InnerError::Start(format!(
        //         "Build produced exit code {}",
        //         build
        //     ))));
        // }
        // Ok(())
    }
}

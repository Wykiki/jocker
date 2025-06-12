use std::{fs::canonicalize, path::Path, process::Stdio, sync::Arc};

use jocker_lib::{error::Result, state::State};
use tempfile::{tempdir, TempDir};
use tokio::process::Child;

pub async fn setup() -> (Arc<State>, TempDir) {
    let project_path =
        canonicalize(format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"))).unwrap();
    setup_cargo(&project_path)
        .await
        .unwrap()
        .wait()
        .await
        .unwrap();
    let dir = tempdir().unwrap();
    copy_dir_all(&project_path, &dir).unwrap();
    (
        Arc::new(State::new(true, None, Some(dir.path())).await.unwrap()),
        dir,
    )
}

pub async fn setup_cargo(path: impl AsRef<Path>) -> tokio::io::Result<Child> {
    let mut build = tokio::process::Command::new("cargo");
    build.stdout(Stdio::piped()).stderr(Stdio::piped());
    build.arg("build");
    build.current_dir(path);
    let build = build.spawn()?;
    Ok(build)
}

pub async fn clean(state: Arc<State>, tempdir: TempDir) -> Result<()> {
    if let Ok(state) = Arc::try_unwrap(state) {
        state.clean().await.unwrap();
    }
    drop(tempdir);
    Ok(())
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

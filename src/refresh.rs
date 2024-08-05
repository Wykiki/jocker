use std::{collections::HashMap, fs::File, hash::Hash, io::BufWriter, process::Command, sync::Arc};

use argh::FromArgs;

use crate::{
    common::{Exec, Process},
    error::{Error, InnerError, Result},
    export_info::{ExportInfoMinimal, SerializedPackage, TargetKind},
    state::State,
};

#[derive(FromArgs, PartialEq, Debug)]
/// Refresh the list of project's binaries
#[argh(subcommand, name = "refresh")]
pub struct RefreshArgs {}

pub struct Refresh {
    _args: RefreshArgs,
    state: Arc<State>,
}

impl Refresh {
    pub fn new(_args: RefreshArgs, state: Arc<State>) -> Self {
        Refresh { _args, state }
    }

    fn fetch_bins() -> Result<Vec<SerializedPackage>> {
        let metadata = Command::new("cargo")
            .arg("metadata")
            .arg("--format-version=1")
            .output()
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

    fn refresh_binaries(&self) -> Result<()> {
        let binaries = Self::fetch_bins()?;

        let file = File::create(self.state.filename_binaries())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &binaries)
            .map_err(Error::with_context(InnerError::StateIo))?;

        println!("Total binaries: {}", binaries.len());
        Ok(())
    }

    fn refresh_processes(&self) -> Result<()> {
        let binaries = self.state.get_binaries()?;
        let binaries_names: Vec<&str> = binaries.iter().map(SerializedPackage::name).collect();
        let processes = self.state.get_processes()?;
        let processes_names: Vec<&str> = processes.iter().map(Process::name).collect();
        let new_binaries_names = Self::keep_unique(binaries_names, processes_names);

        let mut new_processes = processes.clone();
        for name in &new_binaries_names {
            new_processes.push(Process::new(name, name));
        }
        let file = File::create(self.state.filename_processes())?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &new_processes)
            .map_err(Error::with_context(InnerError::StateIo))?;

        println!("Total processes: {}", new_processes.len());
        if !new_binaries_names.is_empty() {
            println!(
                "Added {} new binaries to the processes list",
                new_binaries_names.len()
            );
        }
        Ok(())
    }

    fn keep_unique<T: Eq + Hash>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> {
        a.append(&mut b);
        let init: HashMap<T, usize> = HashMap::new();
        a.into_iter()
            .fold(init, |mut acc, elem| {
                *acc.entry(elem).or_insert(0) += 1;
                // let counter = acc.entry(elem).or_insert(0);
                // acc.insert(elem, *counter + 1 as usize);
                acc
            })
            .into_iter()
            .filter(|elem| elem.1 == 1)
            .map(|elem| elem.0)
            .collect()
    }
}

impl Exec for Refresh {
    fn exec(&self) -> Result<()> {
        self.refresh_binaries()?;
        self.refresh_processes()
    }
}

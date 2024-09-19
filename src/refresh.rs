use std::{collections::HashMap, hash::Hash, process::Command, sync::Arc};

use argh::FromArgs;

use crate::{
    common::{ConfigFile, Exec, Process},
    error::{Error, InnerError, Result},
    export_info::{BinaryPackage, ExportInfoMinimal, SerializedPackage, TargetKind},
    state::State,
};

#[derive(Debug, FromArgs, PartialEq)]
/// Refresh the list of project's binaries
#[argh(subcommand, name = "refresh")]
pub struct RefreshArgs {
    /// whether to refresh based on timestamp or not
    #[argh(switch, short = 's')]
    soft: bool,
}

pub struct Refresh {
    args: RefreshArgs,
    state: Arc<State>,
}

impl Refresh {
    pub fn new(args: RefreshArgs, state: Arc<State>) -> Self {
        Refresh { args, state }
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

    fn refresh(&self) -> Result<()> {
        self.refresh_binaries()?;
        self.refresh_processes()
    }

    fn refresh_binaries(&self) -> Result<()> {
        if self.args.soft {
            return Ok(());
        }
        let binaries: Vec<BinaryPackage> =
            Self::fetch_bins()?.into_iter().map(Into::into).collect();
        let binaries_len = binaries.len();
        self.state.set_binaries(binaries)?;

        println!("Total binaries: {}", binaries_len);
        Ok(())
    }

    fn refresh_processes(&self) -> Result<()> {
        let processes = if let Some(rocker_config) = ConfigFile::load()? {
            rocker_config
                .processes
                .into_iter()
                .map(Into::into)
                .collect()
        } else {
            self.state
                .get_binaries()?
                .into_iter()
                .map(|b| Process::new(&b.name, &b.name))
                .collect()
        };
        self.state.set_processes(processes)?;

        // println!("Total processes: {}", new_processes_len);
        // if !new_binaries_names.is_empty() {
        //     println!(
        //         "Added {} new binaries to the processes list",
        //         new_binaries_names.len()
        //     );
        // }
        Ok(())
    }

    // fn keep_unique<T: Eq + Hash>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> {
    //     a.append(&mut b);
    //     let init: HashMap<T, usize> = HashMap::new();
    //     a.into_iter()
    //         .fold(init, |mut acc, elem| {
    //             *acc.entry(elem).or_insert(0) += 1;
    //             // let counter = acc.entry(elem).or_insert(0);
    //             // acc.insert(elem, *counter + 1 as usize);
    //             acc
    //         })
    //         .into_iter()
    //         .filter(|elem| elem.1 == 1)
    //         .map(|elem| elem.0)
    //         .collect()
    // }
}

impl Exec for Refresh {
    async fn exec(&self) -> Result<()> {
        self.refresh()
    }
}

mod export_info;

use std::{
    env,
    io::{stdout, Result},
};

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    style::Stylize,
    widgets::Paragraph,
    Terminal,
};

use cargo::core::resolver::features::CliFeatures;
use cargo::core::Workspace;
use cargo::ops::OutputMetadataOptions;
use cargo::GlobalContext;
use std::collections::BTreeSet;
use std::path::Path;
use std::rc::Rc;

use crate::export_info::{ExportInfo, SerializedPackage, TargetKind};

fn main() -> Result<()> {
    // stdout().execute(EnterAlternateScreen)?;
    // enable_raw_mode()?;
    // let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    // terminal.clear()?;

    // loop {
    //     terminal.draw(|frame| {
    //         let area = frame.size();
    //         frame.render_widget(
    //             Paragraph::new("Hello Ratatui! (press 'q' to quit)")
    //                 .white()
    //                 .on_blue(),
    //             area,
    //         );
    //     })?;
    //     if event::poll(std::time::Duration::from_millis(16))? {
    //         if let event::Event::Key(key) = event::read()? {
    //             if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
    //                 break;
    //             }
    //         }
    //     }
    // }
    let gctx = GlobalContext::default().unwrap();
    let ws = Workspace::new(
        Path::new(&format!(
            "{}/Cargo.toml",
            &env::var("PWD").expect("Environment variable PWD not defined !")
        )),
        &gctx,
    )
    .unwrap();

    let options = OutputMetadataOptions {
        cli_features: CliFeatures {
            features: Rc::new(BTreeSet::new()),
            all_features: false,
            uses_default_features: true,
        },
        no_deps: false,
        filter_platforms: vec![],
        version: 1,
    };
    let info = cargo::ops::output_metadata(&ws, &options).unwrap();
    let info = serde_json::to_string(&info).unwrap();
    let info: ExportInfo = serde_json::from_str(&info).unwrap();
    let bins: Vec<SerializedPackage> = info
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
    bins.iter().for_each(|bin| println!("{}", bin.name));

    // stdout().execute(LeaveAlternateScreen)?;
    // disable_raw_mode()?;
    Ok(())
}

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use crossterm::event::KeyCode;
use jocker_lib::logs::{Logs, LogsArgs};
use jocker_lib::{logs::LogLine, state::State};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
};
use tokio::{sync::broadcast::Sender, task::JoinHandle};
use tracing::{error, trace};

use crate::ui::event::UiEvent;

use super::JockerWidget;

#[derive(Debug, Default)]
struct UiLogLine {
    process: String,
    line: String,
}

impl From<LogLine> for UiLogLine {
    fn from(value: LogLine) -> Self {
        Self {
            process: value.process,
            line: value.line,
        }
    }
}

impl From<&UiLogLine> for Row<'_> {
    fn from(log_line: &UiLogLine) -> Self {
        Row::new(vec![log_line.process.clone(), log_line.line.clone()])
    }
}

#[derive(Debug, Default)]
struct LogState {
    table_state: TableState,
    logs: VecDeque<UiLogLine>,
    active: bool,
    log_handle: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub(super) struct LogWidget {
    state: Arc<RwLock<LogState>>,
    jocker: Arc<State>,
    event_tx: Sender<UiEvent>,
}

impl LogWidget {
    pub(super) fn new(jocker: Arc<State>, event_tx: Sender<UiEvent>) -> Self {
        Self {
            state: Default::default(),
            jocker,
            event_tx,
        }
    }

    fn run(&self) {
        trace!("LogWidget::run");
        let this = self.clone();
        let log_handle = tokio::spawn(this.fetch_logs(vec![]));
        let this = self.clone();
        tokio::spawn(this.handle_event());
        self.state
            .write()
            .inspect_err(|e| error!("{e}"))
            .unwrap()
            .log_handle = Some(log_handle);
        // TODO: Do not block here, have asynchronous propagation of fetch
        // while !handle.is_finished() {
        //     sleep(Duration::from_millis(10));
        // }
    }

    async fn fetch_logs(self, processes: Vec<String>) {
        trace!("LogWidget::fetch_logs");
        let jocker_state = self.jocker;
        let (_log_handle, mut rx) = Logs::new(
            LogsArgs {
                follow: true,
                processes,
                ..Default::default()
            },
            jocker_state,
        )
        .run()
        .await
        .expect("Cannot fetch logs");
        trace!("LogWidget::fetch_logs will loop");

        while let Some(log_line) = rx.recv().await {
            trace!(
                "Received LogLine for process {}: {}",
                log_line.process,
                log_line.line
            );
            if let Err(e) = self.event_tx.send(UiEvent::NewLogLine(log_line)) {
                error!("{e}");
            }
        }
        trace!("LogWidget::fetch_logs end");
    }

    async fn handle_event(self) {
        while let Ok(event) = self.event_tx.subscribe().recv().await {
            trace!("LogWidget::handle_event {event:?}");
            match event {
                UiEvent::SelectedProcesses(processes) => {
                    let mut state = self.state.write().inspect_err(|e| error!("{e}")).unwrap();
                    if let Some(log_handle) = &state.log_handle {
                        log_handle.abort();
                    }
                    state.logs.clear();
                    let this = self.clone();
                    let log_handle = tokio::spawn(this.fetch_logs(processes));
                    state.log_handle = Some(log_handle);
                }
                UiEvent::NewLogLine(LogLine { process, line }) => {
                    let mut state = self.state.write().inspect_err(|e| error!("{e}")).unwrap();
                    state.logs.push_back(UiLogLine { process, line });
                }
                UiEvent::Dummy => continue,
            }
        }
    }

    fn scroll_down(&self) {
        self.state.write().unwrap().table_state.scroll_down_by(1);
    }

    fn scroll_up(&self) {
        self.state.write().unwrap().table_state.scroll_up_by(1);
    }

    fn copy_line(&self) {
        // TODO
    }
}

impl JockerWidget for &LogWidget {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match keycode {
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(),
            KeyCode::Char('y') => self.copy_line(),
            _ => (),
        }
    }

    fn refresh(&self) {
        trace!("Refresh LogWidget");
        LogWidget::run(self)
    }

    fn is_active(&self) -> bool {
        self.state.read().unwrap().active
    }

    fn set_active(&self, state: bool) {
        self.state.write().unwrap().active = state;
    }
}

impl Widget for &LogWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        trace!("render LogWidget");
        let state = self.state.read().unwrap();
        // let area = popup_area(area, 60, 20);
        // frame.render_widget(Clear, area); //this clears out the background
        // frame.render_widget(block, area);

        let block = Block::bordered()
            .title("[3] Logs")
            .style(self.block_border_style());

        let header = Row::new(vec!["Process", ""]);

        let rows = state.logs.iter();
        let widths = [Constraint::Max(10), Constraint::Fill(1)];
        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(self.table_row_highlight_style());

        drop(state);
        StatefulWidget::render(
            table,
            area,
            buf,
            &mut self.state.write().unwrap().table_state,
        );
    }
}

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::Arc,
};

use jocker_lib::logs::{Logs, LogsArgs};
use jocker_lib::{logs::LogLine, state::State};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use tokio::{
    sync::{
        broadcast::{self, Sender},
        mpsc, RwLock,
    },
    task::JoinHandle,
};
use tracing::{error, trace, warn};

use crate::ui::{event::UiEvent, JockerWidgetState};

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
pub(super) struct LogState {
    table_state: TableState,
    logs: VecDeque<UiLogLine>,
    active: bool,
    log_handle: Option<JoinHandle<()>>,
}

impl LogState {
    pub(super) async fn spawn(
        self,
        jocker: Arc<State>,
        event_tx: Sender<UiEvent>,
    ) -> Arc<RwLock<Self>> {
        trace!("LogWidget::new");
        let (log_tx, log_rx) = mpsc::channel(64);
        let log_handle = tokio::spawn(Self::fetch_logs(jocker.clone(), log_tx.clone(), vec![]));
        let state: Arc<RwLock<LogState>> = Default::default();
        state.write().await.log_handle = Some(log_handle);
        tokio::spawn(Self::handle_ui_event(
            state.clone(),
            jocker.clone(),
            event_tx.clone(),
            log_tx,
        ));
        tokio::spawn(Self::handle_log_event(
            state.clone(),
            jocker.clone(),
            event_tx,
            log_rx,
        ));
        state
    }

    async fn fetch_logs(
        jocker_state: Arc<State>,
        log_tx: mpsc::Sender<BTreeMap<usize, Vec<u8>>>,
        processes: Vec<String>,
    ) {
        trace!("LogWidget::fetch_logs");
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

        while let Some(log_chunk) = rx.recv().await {
            trace!("Received logs for UI",);
            if let Err(e) = log_tx.send(log_chunk).await {
                error!("{e}");
            }
        }
        trace!("LogWidget::fetch_logs end");
    }

    async fn handle_log_event(
        state: Arc<RwLock<LogState>>,
        jocker: Arc<State>,
        event_tx: broadcast::Sender<UiEvent>,
        mut log_rx: mpsc::Receiver<BTreeMap<usize, Vec<u8>>>,
    ) {
        while let Some(logs_by_pid) = log_rx.recv().await {
            let name_by_pid = jocker
                .get_processes()
                .await
                .unwrap()
                .into_iter()
                .filter_map(|p| p.pid.map(|pid| (pid, p.name)))
                .collect::<HashMap<_, _>>();
            for (pid, log_bytes) in logs_by_pid {
                let str = match String::from_utf8(log_bytes) {
                    Ok(str) => str,
                    Err(e) => {
                        error!("unable to read logs for process with pid {pid}: {e}");
                        continue;
                    }
                };
                let lines = str.lines();
                let name = match name_by_pid.get(&pid) {
                    Some(name) => name.clone(),
                    None => {
                        warn!("unable to get process name for pid {pid}");
                        "".to_owned()
                    }
                };
                let mut state = state.write().await;
                for line in lines {
                    state.logs.push_back(UiLogLine {
                        process: name.clone(),
                        line: line.to_owned(),
                    });
                }
            }
            if let Err(e) = event_tx.send(UiEvent::NewLogs) {
                warn!("unable to send UiEvent::NewLogs event: {e}");
            }
        }
    }

    async fn handle_ui_event(
        state: Arc<RwLock<LogState>>,
        jocker: Arc<State>,
        event_tx: broadcast::Sender<UiEvent>,
        log_tx: mpsc::Sender<BTreeMap<usize, Vec<u8>>>,
    ) {
        let mut event_rx = event_tx.subscribe();
        while let Ok(event) = event_rx.recv().await {
            if let UiEvent::SelectedProcesses(processes) = event {
                let mut state = state.write().await;
                if let Some(log_handle) = &state.log_handle {
                    log_handle.abort();
                }
                state.logs.clear();

                let log_handle =
                    tokio::spawn(Self::fetch_logs(jocker.clone(), log_tx.clone(), processes));
                state.log_handle = Some(log_handle);
            }
        }
    }
}

impl JockerWidgetState for LogState {
    fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Default)]
pub(super) struct LogWidget {}

impl StatefulWidget for &LogWidget {
    type State = LogState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        trace!("render LogWidget");
        let block = Block::bordered()
            .title("[3] Logs")
            .style(state.block_border_style());

        let header = Row::new(vec!["Process", ""]);

        let rows = state.logs.iter();
        let widths = [Constraint::Max(10), Constraint::Fill(1)];
        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(state.table_row_highlight_style());
        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

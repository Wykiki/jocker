use std::sync::Arc;

use jocker_lib::{
    common::{Process, ProcessState as JockerProcessState},
    state::State,
    Pid,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState},
};
use tokio::sync::{
    broadcast::{self, Sender},
    RwLock,
};
use tracing::{error, trace};

use crate::ui::{
    event::{ActiveWidgetEvent, UiEvent},
    JockerWidgetState,
};

#[derive(Debug, Default)]
struct UiProcess {
    name: String,
    state: JockerProcessState,
    pid: Option<Pid>,
    selected: bool,
}

impl UiProcess {
    fn toggle_selected(&mut self) {
        self.selected = !self.selected;
    }
}

impl From<Process> for UiProcess {
    fn from(value: Process) -> Self {
        Self {
            name: value.name,
            state: value.state,
            pid: value.pid,
            selected: false,
        }
    }
}

impl From<&UiProcess> for Row<'_> {
    fn from(process: &UiProcess) -> Self {
        Row::new(vec![
            if process.selected {
                '>'.to_string()
            } else {
                ' '.to_string()
            },
            process.name.clone(),
            process.state.to_string(),
            process.pid.map(|v| v.to_string()).unwrap_or_default(),
        ])
    }
}

#[derive(Debug, Default)]
pub(super) struct ProcessState {
    processes: Vec<UiProcess>,
    table_state: TableState,
    active: bool,
}

impl ProcessState {
    pub(super) fn spawn(self, jocker: Arc<State>, event_tx: Sender<UiEvent>) -> Arc<RwLock<Self>> {
        let state: Arc<RwLock<ProcessState>> = Arc::new(RwLock::new(Self {
            active: true,
            ..Default::default()
        }));
        tokio::spawn(Self::fetch_processes(
            state.clone(),
            jocker.clone(),
            event_tx.clone(),
            None,
        ));
        tokio::spawn(Self::handle_event(state.clone(), jocker.clone(), event_tx));
        state
    }

    async fn handle_event(
        state: Arc<RwLock<ProcessState>>,
        jocker: Arc<State>,
        event_tx: broadcast::Sender<UiEvent>,
    ) {
        let mut event_rx = event_tx.subscribe();
        while let Ok(event) = event_rx.recv().await {
            let produced_event = match event {
                UiEvent::ActiveWidget(active_event) if state.read().await.active => {
                    let mut state = state.write().await;
                    Some(match active_event {
                        ActiveWidgetEvent::Down => Self::scroll_down(&mut state.table_state),
                        ActiveWidgetEvent::Up => Self::scroll_up(&mut state.table_state),
                        ActiveWidgetEvent::Select => Self::toggle_select(&mut state),
                    })
                }
                UiEvent::FetchedProcesses => Some(UiEvent::SelectedProcesses(vec![])),
                UiEvent::SelectProcessWidget => {
                    state.write().await.active = true;
                    Some(UiEvent::RenderNeeded)
                }
                UiEvent::SelectStackWidget => {
                    state.write().await.active = false;
                    Some(UiEvent::RenderNeeded)
                }
                UiEvent::SelectedStack(stack) => {
                    Self::fetch_processes(
                        state.clone(),
                        jocker.clone(),
                        event_tx.clone(),
                        Some(stack),
                    )
                    .await;
                    None
                }
                _ => None,
            };
            if let Some(event) = produced_event {
                if let Err(e) = event_tx.send(event) {
                    error!("unable to send ui event from ProcessWidget::handle_event: {e}");
                }
            }
        }
    }

    async fn fetch_processes(
        state: Arc<RwLock<ProcessState>>,
        jocker: Arc<State>,
        event_tx: broadcast::Sender<UiEvent>,
        stack: Option<String>,
    ) {
        if stack.is_some() {
            if let Err(e) = jocker.set_current_stack(&stack).await {
                error!("unable to set current stack: {e}");
            }
        }
        match jocker.filter_processes(&[]).await {
            Ok(processes) => {
                let processes = processes
                    .into_iter()
                    .map(UiProcess::from)
                    .collect::<Vec<_>>();
                let mut state = state.write().await;
                state.processes = processes;
                if !state.processes.is_empty() {
                    state.table_state.select(Some(0));
                }
                if let Err(e) = event_tx.send(UiEvent::FetchedProcesses) {
                    error!("unable to send ui event from ProcessWidget::fetch_processes: {e}");
                }
            }
            Err(e) => error!("unable to fetch processes: {e}"),
        }
    }

    fn scroll_down(table_state: &mut TableState) -> UiEvent {
        table_state.scroll_down_by(1);
        UiEvent::RenderNeeded
    }

    fn scroll_up(table_state: &mut TableState) -> UiEvent {
        table_state.scroll_up_by(1);
        UiEvent::RenderNeeded
    }

    fn toggle_select(state: &mut ProcessState) -> UiEvent {
        trace!("ProcessWidget::toggle_select");
        if let Some(index) = state.table_state.selected() {
            let processes_len = state.processes.len();
            state.processes[index % processes_len].toggle_selected();
        }
        let selected_processes = state
            .processes
            .iter()
            .filter(|process| process.selected)
            .map(|process| process.name.clone())
            .collect();
        UiEvent::SelectedProcesses(selected_processes)
    }
}

impl JockerWidgetState for ProcessState {
    fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Default)]
pub(super) struct ProcessWidget {}

impl StatefulWidget for &ProcessWidget {
    type State = ProcessState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        trace!("render ProcessWidget");
        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered()
            .title("[1] Processes")
            .style(state.block_border_style());

        // a table with the list of pull requests
        let header = Row::new(vec![
            Cell::from(""),
            Cell::from("Process"),
            Cell::from("State"),
            Cell::from("PID"),
        ]);
        let widths = [
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Max(10),
        ];
        let table = Table::new(state.processes.iter(), widths)
            .block(block)
            .header(header)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(state.table_row_highlight_style());

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

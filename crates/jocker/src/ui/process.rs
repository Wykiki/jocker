use std::sync::{Arc, RwLock};

use crossterm::event::KeyCode;
use jocker_lib::{
    common::{Process, ProcessState},
    error::Error,
    state::State,
    Pid,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
};
use tracing::trace;

use super::JockerWidget;

#[derive(Debug, Default)]
struct UiProcess {
    name: String,
    state: ProcessState,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum LoadingState {
    #[default]
    Idle,
    Loaded,
    Error(String),
}

#[derive(Debug, Default)]
struct ProcessesState {
    processes: Vec<UiProcess>,
    loading_state: LoadingState,
    table_state: TableState,
    active: bool,
}

#[derive(Clone)]
pub(super) struct ProcessWidget {
    state: Arc<RwLock<ProcessesState>>,
    jocker: Arc<State>,
}

impl ProcessWidget {
    pub(super) fn new(jocker: Arc<State>) -> Self {
        Self {
            state: Default::default(),
            jocker,
        }
    }

    fn run(&self) {
        let this = self.clone();
        tokio::spawn(this.fetch_processes());
    }

    async fn fetch_processes(self) {
        // this runs once, but you could also run this in a loop, using a channel that accepts
        // messages to refresh on demand, or with an interval timer to refresh every N seconds
        match self.jocker.filter_processes(&[]).await {
            Ok(processes) => self.on_load(processes).await,
            Err(err) => self.on_err(&err).await,
        }
    }

    async fn on_load(&self, processes: Vec<Process>) {
        let processes = processes.into_iter().map(UiProcess::from).collect();
        let mut state = self.state.write().unwrap();
        state.loading_state = LoadingState::Loaded;
        state.processes = processes;
        if !state.processes.is_empty() {
            state.table_state.select(Some(0));
        }
    }

    async fn on_err(&self, err: &Error) {
        self.set_loading_state(LoadingState::Error(err.to_string()));
    }

    fn set_loading_state(&self, state: LoadingState) {
        self.state.write().unwrap().loading_state = state;
    }

    fn scroll_down(&self) {
        self.state.write().unwrap().table_state.scroll_down_by(1);
    }

    fn scroll_up(&self) {
        self.state.write().unwrap().table_state.scroll_up_by(1);
    }

    fn toggle_select(&self) {
        let mut state = self.state.write().unwrap();
        if let Some(index) = state.table_state.selected() {
            state.processes[index].toggle_selected();
        }
    }
}

impl JockerWidget for &ProcessWidget {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match keycode {
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(),
            KeyCode::Enter => self.toggle_select(),
            _ => (),
        }
    }

    fn refresh(&self) {
        ProcessWidget::run(self)
    }

    fn is_active(&self) -> bool {
        self.state.read().unwrap().active
    }

    fn set_active(&self, state: bool) {
        self.state.write().unwrap().active = state;
    }
}

impl Widget for &ProcessWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        trace!("render ProcessWidget");
        let state = self.state.read().unwrap();
        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered()
            .title("[1] Processes")
            .style(self.block_border_style());

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

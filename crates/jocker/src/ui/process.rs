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
    style::Style,
    widgets::{Block, Cell, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
};

use super::JockerWidget;

#[derive(Debug, Default)]
struct UiProcess {
    name: String,
    state: ProcessState,
    pid: Option<Pid>,
}

impl From<Process> for UiProcess {
    fn from(value: Process) -> Self {
        Self {
            name: value.name,
            state: value.state,
            pid: value.pid,
        }
    }
}

impl From<&UiProcess> for Row<'_> {
    fn from(process: &UiProcess) -> Self {
        Row::new(vec![
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
}

impl JockerWidget for &ProcessWidget {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match keycode {
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(),
            _ => (),
        }
    }

    fn run(&self) {
        ProcessWidget::run(self)
    }
}

impl Widget for &ProcessWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = self.state.write().unwrap();

        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered()
            .title("Processes")
            .title_bottom("j/k to scroll, q to quit");

        // a table with the list of pull requests
        let header = Row::new(vec![
            Cell::from("Process"),
            Cell::from("State"),
            Cell::from("PID"),
        ]);
        let rows = state.processes.iter();
        let widths = [
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Max(10),
        ];
        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().on_blue());

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

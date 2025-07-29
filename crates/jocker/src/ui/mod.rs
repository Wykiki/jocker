use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use jocker_lib::{
    common::{Process, ProcessState},
    error::{Error, Result},
    state::State,
    Pid,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize as _},
    text::Line,
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
    DefaultTerminal, Frame,
};

pub struct UiArgs {}

pub struct Ui {
    should_quit: bool,
    processes: ProcessListWidget,
    args: UiArgs,
    state: Arc<State>,
}

impl Ui {
    const FRAMES_PER_SECOND: f32 = 2.0;

    pub fn new(args: UiArgs, state: Arc<State>) -> Self {
        Self {
            should_quit: false,
            processes: ProcessListWidget {
                state: Default::default(),
                jocker: state.clone(),
            },
            args,
            state,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.processes.run();
        let period = Duration::from_secs_f32(1.0 / Self::FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|frame| self.render(frame))?; },
                Some(Ok(event)) = events.next() => self.handle_term_event(&event),
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        self.processes.run();
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [title_area, body_area] = vertical.areas(frame.area());
        let title = Line::from("Jocker").centered().bold();
        frame.render_widget(title, title_area);
        frame.render_widget(&self.processes, body_area);
    }

    fn handle_term_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    // KeyCode::Char('j') | KeyCode::Down => self.pull_requests.scroll_down(),
                    // KeyCode::Char('k') | KeyCode::Up => self.pull_requests.scroll_up(),
                    _ => {}
                }
            }
        }
    }
}

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
        let process = process.clone();
        Row::new(vec![
            process.name.clone(),
            process.pid.map(|v| v.to_string()).unwrap_or_default(),
            process.state.to_string(),
        ])
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum LoadingState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Debug, Default)]
struct ProcessListState {
    processes: Vec<UiProcess>,
    loading_state: LoadingState,
    table_state: TableState,
}

#[derive(Clone)]
struct ProcessListWidget {
    state: Arc<RwLock<ProcessListState>>,
    jocker: Arc<State>,
}

impl ProcessListWidget {
    fn run(&self) {
        let this = self.clone();
        tokio::spawn(this.fetch_processes());
    }

    async fn fetch_processes(self) {
        // this runs once, but you could also run this in a loop, using a channel that accepts
        // messages to refresh on demand, or with an interval timer to refresh every N seconds
        match self.jocker.get_processes().await {
            Ok(processes) => self.on_load(processes).await,
            Err(err) => self.on_err(&err).await,
        }
    }

    async fn on_load(&self, processes: Vec<Process>) {
        let processes = processes
            .into_iter()
            .map(UiProcess::from)
            .by_ref()
            .map(Into::into)
            .collect();
        let mut state = self.state.write().unwrap();
        state.loading_state = LoadingState::Loaded;
        state.processes = processes;
        // if !state.pull_requests.is_empty() {
        //     state.table_state.select(Some(0));
        // }
    }

    async fn on_err(&self, err: &Error) {
        self.set_loading_state(LoadingState::Error(err.to_string()));
    }

    fn set_loading_state(&self, state: LoadingState) {
        self.state.write().unwrap().loading_state = state;
    }
}

impl Widget for &ProcessListWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = self.state.write().unwrap();

        // a block with a right aligned title with the loading state on the right
        let loading_state = Line::from(format!("{:?}", state.loading_state)).right_aligned();
        let block = Block::bordered()
            .title("Processes")
            .title(loading_state)
            .title_bottom("j/k to scroll, q to quit");

        // a table with the list of pull requests
        let rows = state.processes.iter();
        let widths = [
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Max(49),
        ];
        let table = Table::new(rows, widths)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().on_blue());

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

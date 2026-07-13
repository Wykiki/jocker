use std::{
    sync::{Arc, RwLock},
    thread::sleep,
    time::Duration,
};

use crossterm::event::KeyCode;
use jocker_lib::common::Stack;
use jocker_lib::state::State;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
};
use tracing::trace;

use super::JockerWidget;

#[derive(Debug, Default)]
struct UiStack {
    name: String,
}

impl From<Stack> for UiStack {
    fn from(value: Stack) -> Self {
        Self { name: value.name }
    }
}

impl From<&UiStack> for Row<'_> {
    fn from(process: &UiStack) -> Self {
        Row::new(vec![process.name.clone()])
    }
}

#[derive(Debug, Default)]
struct LogState {
    stacks: Vec<UiStack>,
    table_state: TableState,
    active: bool,
}

#[derive(Clone)]
pub(super) struct LogWidget {
    state: Arc<RwLock<LogState>>,
    jocker: Arc<State>,
}

impl LogWidget {
    pub(super) fn new(jocker: Arc<State>) -> Self {
        Self {
            state: Default::default(),
            jocker,
        }
    }

    fn run(&self) {
        let this = self.clone();
        let handle = tokio::spawn(this.fetch_stacks());
        // TODO: Do not block here, have asynchronous propagation of fetch
        while !handle.is_finished() {
            sleep(Duration::from_millis(10));
        }
    }

    async fn fetch_stacks(self) {
        match self.jocker.get_stacks().await {
            Ok(stacks) => self.on_load(stacks).await,
            Err(err) => todo!("{err}"),
        }
    }

    async fn on_load(&self, stacks: Vec<Stack>) {
        let stacks = stacks.into_iter().map(UiStack::from).collect();
        let mut state = self.state.write().unwrap();
        state.stacks = stacks;
        if !state.stacks.is_empty() {
            state.table_state.select(Some(0));
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

        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered()
            .title("[3] Logs")
            .style(self.block_border_style());

        // a table with the list of pull requests
        let rows = state.stacks.iter();
        let widths = [Constraint::Fill(1)];
        let table = Table::new(rows, widths)
            .block(block)
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

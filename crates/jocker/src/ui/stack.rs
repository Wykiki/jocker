use std::sync::{Arc, RwLock};

use crossterm::event::KeyCode;
use jocker_lib::common::Stack;
use jocker_lib::state::State;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Style, Stylize as _},
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState, Widget},
};

use super::{popup_area, JockerWidget};

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
struct StackState {
    stacks: Vec<UiStack>,
    table_state: TableState,
}

#[derive(Clone)]
pub(super) struct StackWidget {
    state: Arc<RwLock<StackState>>,
    jocker: Arc<State>,
}

impl StackWidget {
    pub(super) fn new(jocker: Arc<State>) -> Self {
        Self {
            state: Default::default(),
            jocker,
        }
    }

    fn run(&self) {
        let this = self.clone();
        tokio::spawn(this.fetch_stacks());
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
}

impl JockerWidget for &StackWidget {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match keycode {
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(),
            _ => (),
        }
    }

    fn run(&self) {
        StackWidget::run(self)
    }
}

impl Widget for &StackWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = self.state.write().unwrap();
        let area = popup_area(area, 60, 20);
        // frame.render_widget(Clear, area); //this clears out the background
        // frame.render_widget(block, area);

        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered().title("Stacks");

        // a table with the list of pull requests
        let rows = state.stacks.iter();
        let widths = [Constraint::Fill(1)];
        let table = Table::new(rows, widths)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_symbol(">>")
            .row_highlight_style(Style::new().on_blue());

        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

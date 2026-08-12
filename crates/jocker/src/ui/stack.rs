use std::sync::Arc;

use jocker_lib::common::Stack;
use jocker_lib::state::State;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    widgets::{Block, HighlightSpacing, Row, StatefulWidget, Table, TableState},
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
struct UiStack {
    name: String,
    selected: bool,
}

impl UiStack {
    fn toggle_selected(&mut self) {
        self.selected = !self.selected;
    }
}

impl From<Stack> for UiStack {
    fn from(value: Stack) -> Self {
        Self {
            name: value.name,
            selected: false,
        }
    }
}

impl From<&UiStack> for Row<'_> {
    fn from(process: &UiStack) -> Self {
        Row::new(vec![process.name.clone()])
    }
}

#[derive(Debug, Default)]
pub(super) struct StackState {
    stacks: Vec<UiStack>,
    table_state: TableState,
    active: bool,
}

impl StackState {
    pub(super) fn spawn(self, jocker: Arc<State>, event_tx: Sender<UiEvent>) -> Arc<RwLock<Self>> {
        let state: Arc<RwLock<StackState>> = Default::default();
        tokio::spawn(Self::fetch_stacks(
            state.clone(),
            jocker.clone(),
            event_tx.clone(),
        ));
        tokio::spawn(Self::handle_event(state.clone(), jocker.clone(), event_tx));
        state
    }

    async fn handle_event(
        state: Arc<RwLock<StackState>>,
        _jocker: Arc<State>,
        event_tx: Sender<UiEvent>,
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
                UiEvent::SelectProcessWidget => {
                    state.write().await.active = false;
                    Some(UiEvent::RenderNeeded)
                }
                UiEvent::SelectStackWidget => {
                    state.write().await.active = true;
                    Some(UiEvent::RenderNeeded)
                }
                _ => None,
            };
            if let Some(event) = produced_event {
                if let Err(e) = event_tx.send(event) {
                    error!("unable to send ui event: {e}");
                }
            }
        }
    }

    async fn fetch_stacks(
        state: Arc<RwLock<StackState>>,
        jocker: Arc<State>,
        event_tx: broadcast::Sender<UiEvent>,
    ) {
        match jocker.get_stacks().await {
            Ok(stacks) => {
                let stacks = stacks.into_iter().map(UiStack::from).collect();
                let mut state = state.write().await;
                state.stacks = stacks;
                if !state.stacks.is_empty() {
                    state.table_state.select(Some(0));
                }
                if let Err(e) = event_tx.send(UiEvent::RenderNeeded) {
                    error!("unable to send ui event: {e}");
                }
            }
            Err(err) => todo!("{err}"),
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

    fn toggle_select(state: &mut StackState) -> UiEvent {
        trace!("StackWidget::toggle_select");
        if let Some(index) = state.table_state.selected() {
            let stacks_len = state.stacks.len();
            state.stacks[index % stacks_len].toggle_selected();
        }
        let selected_processes = state
            .stacks
            .iter()
            .filter(|stack| stack.selected)
            .map(|stack| stack.name.clone())
            .collect();
        UiEvent::SelectedProcesses(selected_processes)
    }
}

impl JockerWidgetState for StackState {
    fn is_active(&self) -> bool {
        self.active
    }
}

#[derive(Clone, Default)]
pub(super) struct StackWidget {}

impl StackWidget {}

impl StatefulWidget for &StackWidget {
    type State = StackState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        trace!("render StackWidget");
        // a block with a right aligned title with the loading state on the right
        let block = Block::bordered()
            .title("[2] Stacks")
            .style(state.block_border_style());

        // a table with the list of pull requests
        let rows = state.stacks.iter();
        let widths = [Constraint::Fill(1)];
        let table = Table::new(rows, widths)
            .block(block)
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(state.table_row_highlight_style());
        StatefulWidget::render(table, area, buf, &mut state.table_state);
    }
}

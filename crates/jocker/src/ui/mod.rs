use std::{sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind};
use futures::StreamExt;
use jocker_lib::state::State;
use log::LogWidget;
use process::ProcessWidget;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    DefaultTerminal, Frame,
};
use stack::StackWidget;
use tokio::{
    sync::{
        broadcast::{self, Receiver, Sender},
        RwLock, RwLockWriteGuard,
    },
    time::Instant,
};
use tracing::{error, trace};

use crate::ui::{
    event::{ActiveWidgetEvent, RenderEvent, UiEvent},
    log::LogState,
    process::ProcessState,
    stack::StackState,
};

mod event;
mod log;
mod process;
mod stack;
mod style;

pub(crate) trait JockerWidgetState {
    fn is_active(&self) -> bool;

    fn block_border_style(&self) -> Style {
        if self.is_active() {
            Style::default().green()
        } else {
            Style::default()
        }
    }

    fn table_row_highlight_style(&self) -> Style {
        if self.is_active() {
            Style::default().on_blue()
        } else {
            Style::default()
        }
    }
}

#[derive(Default)]
struct Widgets {
    process: ProcessWidget,
    stack: StackWidget,
    log: LogWidget,
}

struct States {
    process: Arc<RwLock<ProcessState>>,
    stack: Arc<RwLock<StackState>>,
    log: Arc<RwLock<LogState>>,
}

impl States {
    async fn spawn(jocker: Arc<State>, event_tx: Sender<UiEvent>) -> Self {
        Self {
            process: ProcessState::default().spawn(jocker.clone(), event_tx.clone()),
            stack: StackState::default().spawn(jocker.clone(), event_tx.clone()),
            log: LogState::default()
                .spawn(jocker.clone(), event_tx.clone())
                .await,
        }
    }
}

impl States {
    async fn write(
        &self,
    ) -> (
        RwLockWriteGuard<'_, ProcessState>,
        RwLockWriteGuard<'_, StackState>,
        RwLockWriteGuard<'_, LogState>,
    ) {
        (
            self.process.write().await,
            self.stack.write().await,
            self.log.write().await,
        )
    }
}

pub struct UiLayout {
    processes: Rect,
    stacks: Rect,
    logs: Rect,
    footer: Rect,
}

impl UiLayout {
    pub fn new(frame: &mut Frame) -> Self {
        let [main, footer] = frame.area().layout(&Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
        ]));
        let [left, logs] = main.layout(&Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(2),
        ]));
        let [processes, stacks] = left.layout(&Layout::vertical([
            Constraint::Fill(3),
            Constraint::Fill(1),
        ]));
        Self {
            processes,
            stacks,
            logs,
            footer,
        }
    }
}

pub struct Ui {
    // should_quit: bool,
    widgets: Widgets,
    states: States,
    // active_widget: Arc<WidgetType>,
    event_rx: Receiver<UiEvent>,
    event_tx: Sender<UiEvent>,
}

impl Ui {
    pub async fn spawn(state: Arc<State>) -> Self {
        let (event_tx, event_rx) = broadcast::channel(16);
        let states = States::spawn(state.clone(), event_tx.clone()).await;
        Self {
            states,
            widgets: Default::default(),
            event_rx,
            event_tx,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let _user_event_handle = tokio::spawn(Self::handle_user_event(self.event_tx.clone()));

        let fps = Duration::from_millis(40);
        let mut previous_render = Instant::now();
        self.draw(&mut terminal).await?;
        while let Ok(event) = self.event_rx.recv().await {
            match self.handle_event(event).await {
                Some(RenderEvent::Quit) => break,
                Some(RenderEvent::Render) => {
                    let now = Instant::now();
                    // Render only if previous render was done before fps duration
                    if now - previous_render > fps {
                        self.draw(&mut terminal).await?;
                        previous_render = Instant::now();
                    }
                }
                None => continue,
            }
        }
        Ok(())
    }

    async fn draw(&self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        let (mut process_state, mut stack_state, mut log_state) = self.states.write().await;
        terminal.draw(|frame| {
            self.render(frame, &mut process_state, &mut stack_state, &mut log_state)
        })?;
        Ok(())
    }

    fn render(
        &self,
        frame: &mut Frame,
        process_state: &mut ProcessState,
        stack_state: &mut StackState,
        log_state: &mut LogState,
    ) {
        let layout = UiLayout::new(frame);
        let footer = Line::from("q: quit, j k: navigate")
            .left_aligned()
            .style(Style::new().blue());

        frame.render_stateful_widget(&self.widgets.process, layout.processes, process_state);
        frame.render_stateful_widget(&self.widgets.stack, layout.stacks, stack_state);
        frame.render_stateful_widget(&self.widgets.log, layout.logs, log_state);
        frame.render_widget(footer, layout.footer);
    }

    async fn handle_event(&mut self, event: UiEvent) -> Option<RenderEvent> {
        match event {
            UiEvent::NewLogs
            | UiEvent::SelectProcessWidget
            | UiEvent::SelectStackWidget
            | UiEvent::RenderNeeded => Some(RenderEvent::Render),
            UiEvent::Quit => Some(RenderEvent::Quit),
            UiEvent::SelectedProcesses(_) | UiEvent::ActiveWidget(_) => None,
        }
    }

    async fn handle_user_event(event_tx: Sender<UiEvent>) {
        let mut events = EventStream::new();
        while let Some(Ok(event)) = events.next().await {
            // let event = events.next().fuse().await;
            trace!("received user event {event:?}");
            if let Some(ui_event) = match event {
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char('q'),
                    ..
                }) => Some(UiEvent::Quit),
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char('1'),
                    ..
                }) => Some(UiEvent::SelectProcessWidget),
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char('2'),
                    ..
                }) => Some(UiEvent::SelectStackWidget),
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char('j') | KeyCode::Down,
                    ..
                }) => Some(UiEvent::ActiveWidget(ActiveWidgetEvent::Down)),
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char('k') | KeyCode::Up,
                    ..
                }) => Some(UiEvent::ActiveWidget(ActiveWidgetEvent::Up)),
                Event::Key(KeyEvent {
                    kind: KeyEventKind::Press,
                    code: KeyCode::Char(' '),
                    ..
                }) => Some(UiEvent::ActiveWidget(ActiveWidgetEvent::Select)),
                _ => None,
            } {
                if let Err(e) = event_tx.send(ui_event) {
                    error!("unable to send ui event upon receiving user event: {e}");
                }
            }
        }
        error!("user event loop finished, it should not happen");
    }
}

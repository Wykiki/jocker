use std::{sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
use tracing::{error, trace, warn};

use crate::{
    signal::{shutdown_signal, Shutdown, SHUTDOWN_GRACE_PERIOD},
    ui::{
        event::{ActiveWidgetEvent, RenderEvent, UiEvent},
        log::LogState,
        process::ProcessState,
        stack::StackState,
    },
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
    /// Set when the ui was stopped by a termination signal rather than by the user.
    exit_signal: Option<Shutdown>,
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
            exit_signal: None,
        }
    }

    /// Runs the ui event loop until the user quits or a termination signal is received.
    ///
    /// Returns the signal that caused the shutdown, if any, so the caller can exit with the
    /// matching status code. The terminal is *not* restored here: it stays the caller's
    /// responsibility, as it also owns [`ratatui::init()`].
    pub async fn run(
        mut self,
        mut terminal: DefaultTerminal,
    ) -> color_eyre::Result<Option<Shutdown>> {
        let _user_event_handle = tokio::spawn(Self::handle_user_event(self.event_tx.clone()));
        let _signal_handle = tokio::spawn(Self::handle_shutdown_signal(self.event_tx.clone()));

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
        Ok(self.exit_signal)
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
        let footer = Line::from("q / ctrl-c: quit, j k: navigate")
            .left_aligned()
            .style(Style::new().blue());

        frame.render_stateful_widget(&self.widgets.process, layout.processes, process_state);
        frame.render_stateful_widget(&self.widgets.stack, layout.stacks, stack_state);
        frame.render_stateful_widget(&self.widgets.log, layout.logs, log_state);
        frame.render_widget(footer, layout.footer);
    }

    async fn handle_event(&mut self, event: UiEvent) -> Option<RenderEvent> {
        match event {
            UiEvent::ActiveWidget(_)
            | UiEvent::NewLogs
            | UiEvent::RenderNeeded
            | UiEvent::SelectProcessWidget
            | UiEvent::SelectStackWidget
            | UiEvent::SelectedProcesses(_)
            | UiEvent::SelectedStack(_) => Some(RenderEvent::Render),
            UiEvent::Quit => Some(RenderEvent::Quit),
            UiEvent::Signal(signal) => {
                self.exit_signal = Some(signal);
                Some(RenderEvent::Quit)
            }
        }
    }

    /// Turns a termination signal into a [`UiEvent::Signal`] so the render loop exits cleanly.
    ///
    /// Going through the regular event channel means the shutdown follows the exact same path as a
    /// user pressing `q`, which keeps terminal restoration in a single place.
    ///
    /// As a safety net, if the graceful path has not completed within [`SHUTDOWN_GRACE_PERIOD`], or
    /// if a second signal arrives in the meantime, the terminal is restored from here and the
    /// process exits right away: a stuck task must never leave the user with an unusable terminal.
    /// On the normal path this task is dropped along with the runtime long before that happens.
    async fn handle_shutdown_signal(event_tx: Sender<UiEvent>) {
        let signal = match shutdown_signal().await {
            Ok(signal) => signal,
            Err(e) => {
                error!("unable to listen for termination signals: {e}");
                return;
            }
        };
        warn!("received {signal}, shutting down the ui");
        if let Err(e) = event_tx.send(UiEvent::Signal(signal)) {
            error!("unable to send ui event upon receiving {signal}: {e}");
        }

        tokio::select! {
            () = tokio::time::sleep(SHUTDOWN_GRACE_PERIOD) => {
                error!("ui did not shut down within {SHUTDOWN_GRACE_PERIOD:?}, forcing exit");
            }
            _ = shutdown_signal() => {
                warn!("received a second termination signal, forcing exit");
            }
        }
        // Note that exiting here skips the tracing worker guard, so the messages above may not make
        // it to the log file. Restoring the terminal matters more than logging why.
        ratatui::restore();
        std::process::exit(signal.exit_code());
    }

    async fn handle_user_event(event_tx: Sender<UiEvent>) {
        let mut events = EventStream::new();
        while let Some(Ok(event)) = events.next().await {
            // let event = events.next().fuse().await;
            trace!("received user event {event:?}");
            if let Some(ui_event) = Self::user_event(&event) {
                if let Err(e) = event_tx.send(ui_event) {
                    error!("unable to send ui event upon receiving user event: {e}");
                }
            }
        }
        error!("user event loop finished, it should not happen");
    }

    /// Maps a terminal event to the ui event it triggers, if any.
    fn user_event(event: &Event) -> Option<UiEvent> {
        match event {
            // Raw mode clears `ISIG`, so while the ui runs the terminal driver does not turn Ctrl+C
            // into a `SIGINT`: it reaches us as an ordinary key event and has to quit explicitly.
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            })
            | Event::Key(KeyEvent {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn ctrl_c_quits() {
        assert!(matches!(
            Ui::user_event(&press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(UiEvent::Quit)
        ));
    }

    #[test]
    fn plain_c_does_not_quit() {
        assert!(Ui::user_event(&press(KeyCode::Char('c'), KeyModifiers::NONE)).is_none());
    }

    #[test]
    fn q_quits() {
        assert!(matches!(
            Ui::user_event(&press(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(UiEvent::Quit)
        ));
    }

    #[test]
    fn navigation_keys_are_still_mapped() {
        assert!(matches!(
            Ui::user_event(&press(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(UiEvent::ActiveWidget(ActiveWidgetEvent::Down))
        ));
        assert!(matches!(
            Ui::user_event(&press(KeyCode::Up, KeyModifiers::NONE)),
            Some(UiEvent::ActiveWidget(ActiveWidgetEvent::Up))
        ));
    }

    #[test]
    fn key_release_is_ignored() {
        let mut event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        event.kind = KeyEventKind::Release;
        assert!(Ui::user_event(&Event::Key(event)).is_none());
    }

    #[test]
    fn signal_exit_codes_follow_the_shell_convention() {
        assert_eq!(Shutdown::Interrupt.exit_code(), 130);
        assert_eq!(Shutdown::Terminate.exit_code(), 143);
        assert_eq!(Shutdown::Hangup.exit_code(), 129);
    }
}

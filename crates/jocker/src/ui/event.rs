use jocker_lib::event::{JockerAction, JockerEvent};

use crate::signal::Shutdown;

#[derive(Debug, Clone)]
pub enum UiEvent {
    JockerAction(JockerAction),
    JockerEvent(JockerEvent),
    FetchedProcesses,
    SelectProcessWidget,
    SelectStackWidget,
    SelectedProcesses(Vec<String>),
    SelectedStack(String),
    NewLogs,
    Quit,
    /// A termination signal was received and the ui must shut down.
    Signal(Shutdown),
    RenderNeeded,
    ActiveWidget(ActiveWidgetEvent),
}

#[derive(Debug, Clone)]
pub enum ActiveWidgetEvent {
    Down,
    Up,
    Select,
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderEvent {
    Render,
    Quit(
        /// Set when the ui was stopped by a termination signal rather than by the user.
        Option<Shutdown>,
    ),
}

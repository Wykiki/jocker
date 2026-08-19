use crate::signal::Shutdown;

#[derive(Debug, Clone)]
pub enum UiEvent {
    SelectProcessWidget,
    SelectStackWidget,
    SelectedProcesses(Vec<String>),
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
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RenderEvent {
    Render,
    Quit,
}

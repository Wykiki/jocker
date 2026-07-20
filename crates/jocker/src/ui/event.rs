use jocker_lib::logs::LogLine;

#[derive(Debug, Clone)]
pub enum UiEvent {
    SelectedProcesses(Vec<String>),
    NewLogLine(LogLine),
    Dummy,
}

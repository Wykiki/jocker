use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use jocker_lib::{error::Result, state::State};
use log::LogWidget;
use process::ProcessWidget;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Style,
    text::Line,
    widgets::Widget,
    DefaultTerminal, Frame,
};
use stack::StackWidget;

mod log;
mod process;
mod stack;
mod style;

pub(crate) trait JockerWidget: Widget {
    fn dispatch_keycode(&self, keycode: KeyCode);
    fn refresh(&self);
    fn is_active(&self) -> bool;
    fn set_active(&self, state: bool);

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

enum WidgetType {
    Log(LogWidget),
    Process(ProcessWidget),
    Stack(StackWidget),
}

impl JockerWidget for &WidgetType {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match self {
            WidgetType::Log(widget) => widget.dispatch_keycode(keycode),
            WidgetType::Process(widget) => widget.dispatch_keycode(keycode),
            WidgetType::Stack(widget) => widget.dispatch_keycode(keycode),
        }
    }

    fn refresh(&self) {
        match self {
            WidgetType::Log(widget) => widget.refresh(),
            WidgetType::Process(widget) => widget.refresh(),
            WidgetType::Stack(widget) => widget.refresh(),
        }
    }

    fn is_active(&self) -> bool {
        match self {
            WidgetType::Log(widget) => widget.is_active(),
            WidgetType::Process(widget) => widget.is_active(),
            WidgetType::Stack(widget) => widget.is_active(),
        }
    }

    fn set_active(&self, state: bool) {
        match self {
            WidgetType::Log(widget) => widget.set_active(state),
            WidgetType::Process(widget) => widget.set_active(state),
            WidgetType::Stack(widget) => widget.set_active(state),
        }
    }
}

impl Widget for &WidgetType {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            WidgetType::Log(widget) => widget.render(area, buf),
            WidgetType::Process(widget) => widget.render(area, buf),
            WidgetType::Stack(widget) => widget.render(area, buf),
        }
    }
}

struct Widgets {
    process: Arc<WidgetType>,
    stack: Arc<WidgetType>,
    log: Arc<WidgetType>,
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
    should_quit: bool,
    widgets: Widgets,
    active_widget: Arc<WidgetType>,
}

impl Ui {
    pub fn new(state: Arc<State>) -> Self {
        let log = Arc::new(WidgetType::Log(LogWidget::new(state.clone())));
        let process = Arc::new(WidgetType::Process(ProcessWidget::new(state.clone())));
        let stack = Arc::new(WidgetType::Stack(StackWidget::new(state.clone())));
        let active_widget = process.clone();
        process.as_ref().set_active(true);
        let widgets = Widgets {
            log,
            process,
            stack,
        };
        Self {
            should_quit: false,
            active_widget,
            widgets,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.widgets.process.as_ref().refresh();
        self.widgets.stack.as_ref().refresh();
        let mut events = EventStream::new();

        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            tokio::select! {
                // _ = interval.tick() => { terminal.draw(|frame| self.render(frame))?; },
                Some(Ok(event)) = events.next() => self.handle_term_event(&event),
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        // let [top, main] = frame.area().layout(&vertical);
        // let [left, middle, right] = main.layout(&horizontal);
        let layout = UiLayout::new(frame);

        // let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        // let [title_area, body_area] = vertical.areas(frame.area());
        // let title = Line::from("Jocker").centered().bold();
        let footer = Line::from("q: quit, x: menu, h j k l: navigate")
            .left_aligned()
            .style(Style::new().blue());
        frame.render_widget(self.widgets.process.as_ref(), layout.processes);
        frame.render_widget(self.widgets.stack.as_ref(), layout.stacks);
        frame.render_widget(self.widgets.log.as_ref(), layout.logs);
        frame.render_widget(footer, layout.footer);
    }

    fn handle_term_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('1') => self.activate_widget(self.widgets.process.clone()),
                    KeyCode::Char('2') => self.activate_widget(self.widgets.stack.clone()),
                    keycode => self.dispatch_keycode(keycode),
                }
            }
        }
    }

    fn activate_widget(&mut self, new_active: Arc<WidgetType>) {
        self.active_widget.as_ref().set_active(false);
        self.active_widget = new_active;
        self.active_widget.as_ref().set_active(true);
    }

    fn dispatch_keycode(&self, keycode: KeyCode) {
        self.widget().dispatch_keycode(keycode);
    }

    fn widget(&self) -> &WidgetType {
        &self.active_widget
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

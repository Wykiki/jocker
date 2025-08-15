use std::sync::Arc;

use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use futures::StreamExt;
use jocker_lib::{error::Result, state::State};
use process::ProcessWidget;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize as _,
    text::Line,
    widgets::Widget,
    DefaultTerminal, Frame,
};
use stack::StackWidget;

mod process;
mod stack;

pub(crate) trait JockerWidget: Widget {
    fn dispatch_keycode(&self, keycode: KeyCode);
    fn run(&self);
}

enum WidgetType {
    Process(ProcessWidget),
    Stack(StackWidget),
}

impl JockerWidget for &WidgetType {
    fn dispatch_keycode(&self, keycode: KeyCode) {
        match self {
            WidgetType::Process(widget) => widget.dispatch_keycode(keycode),
            WidgetType::Stack(widget) => widget.dispatch_keycode(keycode),
        }
    }

    fn run(&self) {
        match self {
            WidgetType::Process(widget) => widget.run(),
            WidgetType::Stack(widget) => widget.run(),
        }
    }
}

impl Widget for &WidgetType {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            WidgetType::Process(widget) => widget.render(area, buf),
            WidgetType::Stack(widget) => widget.render(area, buf),
        }
    }
}

struct Widgets {
    process: Arc<WidgetType>,
    stack: Arc<WidgetType>,
}

pub struct Ui {
    should_quit: bool,
    widgets: Widgets,
    active_widget: Arc<WidgetType>,
}

impl Ui {
    pub fn new(state: Arc<State>) -> Self {
        let process = Arc::new(WidgetType::Process(ProcessWidget::new(state.clone())));
        let stack = Arc::new(WidgetType::Stack(StackWidget::new(state.clone())));
        let active_widget = process.clone();
        let widgets = Widgets { process, stack };
        Self {
            should_quit: false,
            active_widget,
            widgets,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.active_widget.as_ref().run();
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
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
        let [title_area, body_area] = vertical.areas(frame.area());
        let title = Line::from("Jocker").centered().bold();
        self.active_widget.as_ref().run();
        frame.render_widget(title, title_area);
        frame.render_widget(self.active_widget.as_ref(), body_area);
    }

    fn handle_term_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('n') => self.toggle_stacks(),
                    keycode => self.dispatch_keycode(keycode),
                }
            }
        }
    }

    fn dispatch_keycode(&self, keycode: KeyCode) {
        self.widget().dispatch_keycode(keycode);
    }

    fn toggle_stacks(&mut self) {
        if matches!(*self.active_widget, WidgetType::Stack(_)) {
            self.active_widget = self.widgets.process.clone();
        } else {
            self.active_widget = self.widgets.stack.clone();
        }
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

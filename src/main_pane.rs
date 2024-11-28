use std::io::Write;

use crossterm::{
    cursor,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use ratatui::{
    layout::{
        Constraint::{Fill, Length, Min},
        Layout, Rect, Size,
    },
    style::Stylize,
    text::Span,
    widgets::{Block, List, Paragraph, StatefulWidget, Wrap},
    Frame,
};

use crate::{scroll_view::ScrollView, scroll_view_state::ScrollViewState};

pub struct MainPane {
    start_position: (u16, u16),
    lines: Vec<String>,
    scroll_view_state: ScrollViewState,
}

const SCROLL_VIEW_HEIGHT: u16 = 200;

impl MainPane {
    pub fn new() -> Self {
        let mut stdout = std::io::stdout();
        let start_position = cursor::position().unwrap();
        enable_raw_mode().unwrap();
        stdout.queue(terminal::EnterAlternateScreen).unwrap();
        stdout
            .queue(terminal::Clear(terminal::ClearType::All))
            .unwrap();
        stdout.queue(cursor::Hide).unwrap();
        stdout.queue(cursor::MoveToRow(0)).unwrap();

        stdout.flush().unwrap();
        let lines = Vec::new();
        Self {
            start_position,
            lines,
            scroll_view_state: ScrollViewState::new(),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());
        let para = self.lines.join("\n");
        let lines = self.lines.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let main_content = List::new(lines);

        // let main_content = Paragraph::new(para);
        let status = format!("No. of lines {} ", self.lines.len());
        frame.render_widget(Block::bordered().title("Filter"), title_area);
        frame.render_widget(main_content, main_area);

        // let mut scroll_view = ScrollView::new(Size::new(main_area.width, SCROLL_VIEW_HEIGHT));
        // scroll_view.render_widget(main_content, scroll_view.area());
        // scroll_view.render(main_area, frame.buffer_mut(), &mut self.scroll_view_state);
        frame.render_widget(Block::bordered().title(status.as_str()), status_area);
    }

    pub fn scroll_up(&mut self) {
        self.scroll_view_state.scroll_up();
    }

    pub fn scroll_down(&mut self) {
        self.scroll_view_state.scroll_down();
    }

    pub fn add_line(&mut self, s: impl Into<String>) {
        self.lines.push(s.into());
    }
}

impl Drop for MainPane {
    fn drop(&mut self) {
        disable_raw_mode().unwrap();
        let mut stdout = std::io::stdout();
        stdout.queue(terminal::LeaveAlternateScreen).unwrap();
        stdout
            .queue(cursor::MoveTo(self.start_position.0, self.start_position.1))
            .unwrap();
        stdout.queue(cursor::Show).unwrap();
        stdout.flush().unwrap();
    }
}

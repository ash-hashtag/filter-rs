use std::io::Write;

use crossterm::{
    cursor,
    terminal::{self, disable_raw_mode, enable_raw_mode},
    QueueableCommand,
};
use ratatui::{
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    widgets::Block,
    Frame,
};

use crate::pages::Pages;

pub struct MainPane {
    start_position: (u16, u16),
    pages: Pages,
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
        Self {
            start_position,
            pages: Pages::new(40_000, 10),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());

        let status = format!("No. of lines {} ", self.pages.len());
        frame.render_widget(Block::bordered().title("Filter"), title_area);
        frame.render_widget(Block::bordered().title(status.as_str()), status_area);
    }

    pub fn add_line(&mut self, s: &str) {
        self.pages.add_line(s)
    }

    pub fn scroll_up(&self) {
        todo!()
    }

    pub fn scroll_down(&self) {
        todo!()
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

use std::io::Write;

use crossterm::{cursor, terminal, QueueableCommand};

use crate::pages::Pages;

#[derive(Default, Copy, Clone)]
pub struct Vec2 {
    x: u16,
    y: u16,
}
impl Vec2 {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

pub struct State {
    pages: Pages,
    cursor_position: Vec2,
    current_search_term: String,
    current_found_lines: Vec<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            pages: Pages::default(),
            cursor_position: Vec2::new(0, 0),
            current_search_term: String::new(),
            current_found_lines: Vec::new(),
        }
    }

    #[inline]
    pub fn size(&self) -> (u16, u16) {
        terminal::size().unwrap()
    }

    #[inline]
    pub fn width(&self) -> u16 {
        self.size().0
    }

    #[inline]
    pub fn height(&self) -> u16 {
        self.size().1 - 1
    }

    #[inline]
    pub fn cursor_pos(&self) -> Vec2 {
        self.cursor_position
    }

    pub fn add_line(&mut self, s: &str) -> anyhow::Result<()> {
        self.pages.add_line(s);
        let mut stdout = std::io::stdout();
        stdout.write(s.as_bytes())?;
        stdout.queue(cursor::MoveToColumn(0))?;
        if !self.current_search_term.is_empty() && s.contains(&self.current_search_term) {
            self.current_found_lines.push(s.into());
        }
        Ok(())
    }

    pub fn move_cursor(&mut self, x: i16, y: i16) -> anyhow::Result<()> {
        let (_, cy) = cursor::position().unwrap();
        let mut stdout = std::io::stdout();

        if y < 0 && cy == self.height() {
            stdout.queue(terminal::ScrollDown(-y as u16))?;
        } else if y > 0 && cy == 0 {
            stdout.queue(terminal::ScrollUp(y as u16))?;
        }

        if x > 0 {
            stdout.queue(cursor::MoveRight(x as u16))?;
        } else if x < 0 {
            stdout.queue(cursor::MoveLeft(-x as u16))?;
        }

        if y > 0 {
            stdout.queue(cursor::MoveUp(y as u16))?;
        } else if y < 0 {
            stdout.queue(cursor::MoveDown(-y as u16))?;
        }

        Ok(())
    }

    // To Clear put an empty string
    pub fn set_search_string(&mut self, s: impl Into<String>) {
        self.current_search_term = s.into();
        self.current_found_lines.clear();
    }
}

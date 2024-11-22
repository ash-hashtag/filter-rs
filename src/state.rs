use std::io::Write;

use crossterm::{cursor, terminal, QueueableCommand};

const PAGE_SIZE: usize = 1024 * 1024 * 4; // 4 MB
const MAX_PAGES: usize = 10; // 40 MB

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
    pages: Vec<String>,
    cursor_position: Vec2,
}

impl State {
    pub fn new() -> Self {
        let mut v = Vec::with_capacity(MAX_PAGES);
        v.push(String::with_capacity(PAGE_SIZE));
        Self {
            pages: v,
            cursor_position: Vec2::new(0, 0),
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
        let last_page = self.pages.last().unwrap();
        if last_page.len() + s.len() > last_page.capacity() {
            if self.pages.len() >= MAX_PAGES {
                self.pages.remove(0);
            }
            self.pages.push(String::with_capacity(PAGE_SIZE));
        }
        self.pages.last_mut().unwrap().push_str(s);
        let mut stdout = std::io::stdout();
        // stdout.queue(cursor::MoveTo(
        //     self.cursor_position.x,
        //     self.cursor_position.y,
        // ))?;
        stdout.write(s.as_bytes())?;
        stdout.queue(cursor::MoveToColumn(0))?;

        // let (x, y) = cursor::position()?;
        // self.cursor_position = Vec2::new(x, y);

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

    pub fn search_string(&self, s: &str) -> anyhow::Result<Vec<String>> {
        let area = self.width() * self.height();

        for page in self.pages.iter().rev() {
            for line in page.lines().rev() {}
        }
    }
}

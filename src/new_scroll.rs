use std::{
    ops::{Index, Range},
    slice::SliceIndex,
};

use ratatui::{buffer::Buffer, style::Style};

use crate::pages::Page;

#[derive(Debug, Clone, Copy)]
pub enum InstructionQueue {
    None,
    Up,
    Down,
}

pub struct PageAndView {
    page: Page,
    view: Range<usize>,
}

pub struct PageScrollState<'a> {
    page: &'a Page,
    page_view: Range<usize>,
    auto_scroll: bool,
    queue: InstructionQueue,
    // previous_drawn_page: Page,
    current_lines_in_reverse: Page,
    current_visible_lines: PageAndView,
}

impl<'a> PageScrollState<'a> {
    pub fn get_lines_in_reverse(&mut self, width: usize, height: usize) {
        // let mut lines_to_draw = Page::with_capacities(height * (width + 8), height);
        // std::mem::swap(&mut self.previous_drawn_page, &mut self.current_draw_page);
        let mut lines_to_draw = Page::with_capacities(width * (10 + height), height + 10);
        // lines_to_draw.clear();

        for i in self.page_view.clone().into_iter().rev() {
            self.page_view.start = i;

            let lines = textwrap::wrap(&self.page[i], width);

            for l in lines.iter().rev() {
                lines_to_draw.add_line(&l);
            }
            if height >= lines_to_draw.len() {
                break;
            }
        }
    }

    pub fn apply_queue(&mut self) {
        match self.queue {
            InstructionQueue::None => {
                if self.auto_scroll {
                    self.page_view.end = self.page.len();
                }
            }
            InstructionQueue::Up => {
                if self.page_view.end < self.page.len() {
                    self.page_view.start += 1;
                    self.page_view.end += 1;
                }
            }
            InstructionQueue::Down => {
                if self.page_view.start > 0 {
                    self.page_view.start -= 1;
                    self.page_view.end -= 1;
                }
            }
        }

        self.queue = InstructionQueue::None;
    }

    pub fn requires_redraw(&mut self) -> bool {
        if !matches!(self.queue, InstructionQueue::None) {
            // let mut next = self.clone();
            // next.apply_queue();
            // if next.page_view == self.page_view {
            //     return false;
            // } else {
            //     return true;
            // }
        }
        false
    }

    pub fn draw(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let height = area.height;
        let width = area.width;

        self.get_lines_in_reverse(width as usize, height as usize);

        let lines_in_reverse = &self.current_lines_in_reverse;

        for i in 0..lines_in_reverse.len() {
            let y = i as u16;
            let idx = lines_in_reverse.len() - i - 1;
            let line = &lines_in_reverse[i];
            buffer.set_string(0, y, line, Style::new());
        }
    }
}

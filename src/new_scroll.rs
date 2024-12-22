use std::{
    collections::LinkedList,
    ops::{Index, Range},
    slice::SliceIndex,
};

use ratatui::{buffer::Buffer, style::Style};

use crate::pages::{Page, PageLineIterator};

#[derive(Debug, Clone, Copy, Default)]
pub enum InstructionQueue {
    #[default]
    None,
    Up,
    Down,
    Resize {
        width: usize,
        height: usize,
    },
}

#[derive(Default, Debug)]
pub struct LineWithIdx {
    idx: usize,
    line: String,
}

#[derive(Default, Debug)]
pub struct LinesToRenderAndView {
    lines: LinkedList<LineWithIdx>,
    view: Range<usize>,
}

#[derive(Default)]
pub struct PageAndView {
    page: Page,
    view: Range<usize>,
}

#[derive(Default)]
pub struct PageScrollState {
    page: Page,
    page_view: Range<usize>,
    auto_scroll: bool,

    current_lines_in_reverse: PageAndView,
    lines_being_drawn: LinesToRenderAndView,

    width: usize,
    height: usize,
}

impl PageScrollState {
    pub fn get_lines_to_render(&mut self, is_width_changed: bool) {
        let (width, height) = (self.width, self.height);
        let mut visible_lines = LinkedList::new();

        if is_width_changed {
            for i in (0..self.page_view.end).into_iter().rev() {
                self.page_view.start = i;
                let lines = textwrap::wrap(&self.page[i], width);

                for l in lines.iter().rev() {
                    visible_lines.push_front(LineWithIdx {
                        idx: i,
                        line: l.to_string(),
                    });
                }
                if height <= visible_lines.len() {
                    break;
                }
            }
        } else {
            let previous_lines =
                std::mem::replace(&mut self.lines_being_drawn.lines, LinkedList::new());

            let mut iter = previous_lines.into_iter().rev();

            let mut end = self.page_view.end;
            while let Some(item) = iter.next() {
                if item.idx <= self.page_view.end {
                    end = item.idx;
                    visible_lines.push_front(item);
                    break;
                }
            }

            for i in (0..end).into_iter().rev() {
                self.page_view.start = i;

                let lines = textwrap::wrap(&self.page[i], width);

                for l in lines.iter().rev() {
                    visible_lines.push_front(LineWithIdx {
                        idx: i,
                        line: l.to_string(),
                    });
                }
                if height <= visible_lines.len() {
                    break;
                }
            }
        }

        self.lines_being_drawn = LinesToRenderAndView {
            view: (visible_lines.len().saturating_sub(height))..visible_lines.len(),
            lines: visible_lines,
        }
    }

    /// returns if the instruction requires redraw
    pub fn apply_queue(&mut self, queue: InstructionQueue) -> bool {
        let mut requires_redraw = false;
        let mut is_width_changed = false;
        if self.auto_scroll {
            if self.page_view.end != self.page.len() {
                self.page_view.end = self.page.len();
                requires_redraw = true;
            }
        }

        match queue {
            InstructionQueue::None => {
                // if requires_redraw {
                //     self.get_lines_to_render(false);
                // }
            }
            InstructionQueue::Up => {
                if self.lines_being_drawn.view.end < self.lines_being_drawn.lines.len() {
                    self.lines_being_drawn.view.start += 1;
                    self.lines_being_drawn.view.end += 1;
                    requires_redraw = true;
                } else {
                    if self.page.len() > self.page_view.end {
                        // self.page_view.start += 1;
                        self.page_view.end += 1;
                        // self.get_lines_to_render(false);
                        requires_redraw = true;
                    }
                }
            }
            InstructionQueue::Down => {
                if self.lines_being_drawn.view.start > 0 {
                    self.lines_being_drawn.view.start -= 1;
                    self.lines_being_drawn.view.end -= 1;
                    requires_redraw = true;
                } else {
                    if self.page_view.start > 0 && self.page_view.end > 0 {
                        self.page_view.end -= 1;
                        // self.get_lines_to_render(false);
                        requires_redraw = true;
                    }
                }
            }
            InstructionQueue::Resize { width, height } => {
                is_width_changed = self.width != width;
                self.width = width;
                self.height = height;
                // self.get_lines_to_render(is_width_changed);
                requires_redraw = true;
            }
        }

        if requires_redraw {
            self.get_lines_to_render(is_width_changed);
        }

        requires_redraw
    }

    // pub fn draw(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
    //     let height = area.height;
    //     let width = area.width;

    //     self.get_lines_in_reverse(width as usize, height as usize);

    //     let lines_in_reverse = &self.current_lines_in_reverse;

    //     for i in 0..lines_in_reverse.page.len() {
    //         let y = i as u16;
    //         let idx = lines_in_reverse.page.len() - i - 1;
    //         let line = &lines_in_reverse.page[i];
    //         buffer.set_string(0, y, line, Style::new());
    //     }
    // }
}

#[test]
fn test_new_scroll() {
    let mut state = PageScrollState::default();
    state.auto_scroll = true;
    let mut buf = String::new();

    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ    -_.,"
        .chars()
        .collect::<Vec<_>>();

    for i in 0..16 {
        let len = rand::random::<usize>() % 64;
        buf.clear();

        for _ in 0..len {
            let c = chars[rand::random::<usize>() % chars.len()];
            buf.push(c);
        }

        state.page.add_line(&buf);
    }

    for (idx, line) in PageLineIterator::new(&state.page).enumerate() {
        print!("{}: '{}'\n", idx, line);
    }

    println!("^^^^^ current lines ^^^^^");

    if state.apply_queue(InstructionQueue::Resize {
        width: 10,
        height: 10,
    }) {
        println!(
            "--------terminal resize (10x10) -------- real length {}, view {:?}",
            state.lines_being_drawn.lines.len(),
            state.lines_being_drawn.view
        );

        let mut iter = state
            .lines_being_drawn
            .lines
            .iter()
            .skip(state.lines_being_drawn.view.start)
            .take(state.lines_being_drawn.view.len());
        for line in iter {
            print!("{:>4}: {}\n", line.idx, line.line);
        }

        println!("------------------------");
    }

    state.auto_scroll = false;

    // state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    state.apply_queue(InstructionQueue::Down);
    if state.apply_queue(InstructionQueue::Down) {
        println!(
            "--------terminal (downx9)-------- real length {}, view {:?}",
            state.lines_being_drawn.lines.len(),
            state.lines_being_drawn.view
        );

        let mut iter = state
            .lines_being_drawn
            .lines
            .iter()
            .skip(state.lines_being_drawn.view.start)
            .take(state.lines_being_drawn.view.len());
        for line in iter {
            print!("{:>4}: {}\n", line.idx, line.line);
        }
        println!("------------------------");
    }
    if state.apply_queue(InstructionQueue::Resize {
        width: 15,
        height: 5,
    }) {
        println!(
            "--------terminal resize(15x5)-------- real length {}, view {:?}",
            state.lines_being_drawn.lines.len(),
            state.lines_being_drawn.view
        );

        let mut iter = state
            .lines_being_drawn
            .lines
            .iter()
            .skip(state.lines_being_drawn.view.start)
            .take(state.lines_being_drawn.view.len());
        for line in iter {
            print!("{:>4}: {}\n", line.idx, line.line);
        }
        println!("------------------------");
    }

    state.auto_scroll = true;
    if state.apply_queue(InstructionQueue::None) {
        println!(
            "--------terminal (autoscroll)-------- real length {}, view {:?}",
            state.lines_being_drawn.lines.len(),
            state.lines_being_drawn.view
        );

        let mut iter = state
            .lines_being_drawn
            .lines
            .iter()
            .skip(state.lines_being_drawn.view.start)
            .take(state.lines_being_drawn.view.len());
        for line in iter {
            print!("{:>4}: {}\n", line.idx, line.line);
        }
        println!("------------------------");
    }

    panic!();
}

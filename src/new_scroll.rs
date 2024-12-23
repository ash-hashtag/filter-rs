use std::{
    collections::LinkedList,
    io::Write,
    ops::{Index, Range},
    slice::SliceIndex,
};

use crossterm::QueueableCommand;
use rand::{Rng, SeedableRng};
use ratatui::{buffer::Buffer, style::Style, widgets::Widget};

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

    JumpTo(usize),
}

#[derive(Default, Debug)]
pub struct LineWithIdx {
    pub idx: usize,
    pub line: String,
}

#[derive(Default, Debug)]
pub struct LinesToRenderAndView {
    pub lines: LinkedList<LineWithIdx>,
    pub view: Range<usize>,
}

#[derive(Default)]
pub struct PageAndView {
    page: Page,
    view: Range<usize>,
}

#[derive(Default)]
pub struct PageScrollState {
    pub show_line_numbers: bool,
    page: Page,
    page_view: Range<usize>,
    pub auto_scroll: bool,
    lines_being_drawn: LinesToRenderAndView,
    width: usize,
    height: usize,
    pub requires_redraw: bool,
}

impl PageScrollState {
    pub fn add_line(&mut self, line: &str) {
        self.page.add_line(line);
    }

    /// returns if the instruction requires redraw    
    pub fn apply_queue(&mut self, queue: InstructionQueue) {
        let (width, height) = (self.width, self.height);

        let mut previous_lines = &mut self.lines_being_drawn.lines;

        let previous_start = previous_lines
            .front()
            .and_then(|x| Some(x.idx))
            .unwrap_or(0);
        let previous_end = previous_lines
            .back()
            .and_then(|x| Some(x.idx))
            .unwrap_or(self.page_view.end);

        match queue {
            InstructionQueue::None => {
                if self.auto_scroll {
                    if self.page_view.end != self.page.len() {
                        self.page_view.end = self.page.len();

                        let mut last_lines = LinkedList::new();

                        let range = if previous_end != 0 {
                            (previous_end + 1..self.page_view.end)
                        } else {
                            (previous_end..self.page_view.end)
                        };
                        log::info!(
                            "auto scrolling to end adding lines of previous_end_idx: {} page_view: {:?}",
                            previous_end,
                            range
                        );

                        for i in range.into_iter().rev() {
                            let lines = textwrap::wrap(&self.page[i], width);
                            for l in lines.into_iter().rev() {
                                last_lines.push_front(LineWithIdx {
                                    idx: i,
                                    line: l.to_string(),
                                });
                            }

                            if height <= last_lines.len() {
                                self.page_view.start = i;
                                self.lines_being_drawn.view =
                                    last_lines.len().saturating_sub(height)..last_lines.len();
                                self.lines_being_drawn.lines = last_lines;

                                self.requires_redraw = true;
                                return;
                            }
                        }

                        {
                            let back = previous_lines.back();
                            let front = last_lines.front();

                            log::info!("Joining linked list at {:?} <> {:?}", back, front);
                        }

                        previous_lines.append(&mut last_lines);
                        let start = previous_lines.len().saturating_sub(height);

                        let start_idx = previous_lines
                            .iter()
                            .nth(start)
                            .and_then(|x| Some(x.idx))
                            .unwrap_or(0);

                        self.page_view.start = start_idx;
                        while let Some(item) = previous_lines.pop_front() {
                            if item.idx >= start_idx {
                                self.lines_being_drawn.lines.push_front(item);
                                break;
                            }
                        }

                        self.lines_being_drawn.view =
                            (self.lines_being_drawn.lines.len().saturating_sub(height))
                                ..(self.lines_being_drawn.lines.len());

                        self.requires_redraw = true;
                        return;
                    } else if self.lines_being_drawn.view.end != self.lines_being_drawn.lines.len()
                    {
                        self.page_view.start = self
                            .lines_being_drawn
                            .lines
                            .front()
                            .and_then(|x| Some(x.idx))
                            .unwrap_or(0);
                        self.lines_being_drawn.view =
                            (self.lines_being_drawn.lines.len().saturating_sub(height))
                                ..(self.lines_being_drawn.lines.len());

                        self.requires_redraw = true;
                    }
                }
            }
            InstructionQueue::Up => {
                if self.lines_being_drawn.view.end < previous_lines.len() {
                    self.lines_being_drawn.view.start += 1;
                    self.lines_being_drawn.view.end += 1;
                    self.requires_redraw = true;
                    return;
                }
                if self.page.len() > self.page_view.end {
                    // self.page_view.start += 1;
                    self.page_view.end += 1;
                    // self.get_lines_to_render(false);
                    self.requires_redraw = true;
                }

                /*
                     0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8
                old_start-^      old_end-^         ^-new_end

                  new_end > old_end

                  calculate old_end to new_end in reverse
                  append  (old_start to old_end) to  (old_end to new_end)

                 */
                let mut visible_lines = LinkedList::new();

                let range = (previous_end + 1..(self.page_view.end + 1).min(self.page.len()));

                log::info!("pushing from {:?} lines until filling height", range);

                for i in range.into_iter().rev() {
                    // self.page_view.start = i;
                    let lines = textwrap::wrap(&self.page[i], width);
                    for l in lines.iter().rev() {
                        visible_lines.push_front(LineWithIdx {
                            idx: i,
                            line: l.to_string(),
                        });
                    }
                    // if height <= visible_lines.len() {
                    //     break;
                    // }
                }

                // let mut visible_end = self.lines_being_drawn.view.end + visible_lines.len();
                let mut visible_end = self.lines_being_drawn.view.end + 1;
                log::info!("putting previous lines infront of visible lines");
                previous_lines.append(&mut visible_lines);

                let mut visible_start = visible_end.saturating_sub(height);
                let start_idx = previous_lines
                    .iter()
                    .nth(visible_start)
                    .and_then(|x| Some(x.idx))
                    .unwrap_or(0);
                log::info!("popping front lines with idx < {start_idx}");

                while let Some(item) = previous_lines.pop_front() {
                    if item.idx >= start_idx {
                        self.page_view.start = item.idx;
                        previous_lines.push_front(item);
                        break;
                    } else {
                        visible_start -= 1;
                    }
                }

                self.lines_being_drawn.view =
                    visible_start..(visible_start + height).min(previous_lines.len());
            }
            InstructionQueue::Down => {
                if self.lines_being_drawn.view.start > 0 {
                    self.lines_being_drawn.view.start -= 1;
                    self.lines_being_drawn.view.end -= 1;
                    self.requires_redraw = true;
                    return;
                }
                if self.page_view.start > 0 {
                    self.page_view.start -= 1;
                    self.requires_redraw = true;
                } else {
                    return;
                }

                /*
                     0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8
                old_start-^      new_end-^         ^-old_end

                  new_end > old_end

                  pop until new_end
                  keep (old_start to new_end)
                  calculate from old_start to 0 unil enough lines fill height

                 */

                let mut visible_end = self.lines_being_drawn.view.end - 1;
                log::info!("pushing front lines until filling height");
                for i in (0..previous_start).into_iter().rev() {
                    self.page_view.start = i;
                    let lines = textwrap::wrap(&self.page[i], width);
                    for l in lines.iter().rev() {
                        visible_end += 1;
                        previous_lines.push_front(LineWithIdx {
                            idx: i,
                            line: l.to_string(),
                        });
                    }
                    if height <= previous_lines.len() {
                        break;
                    }
                }

                if let Some(last_visible_item) = previous_lines
                    .iter()
                    .rev()
                    .nth(previous_lines.len() - visible_end)
                {
                    let idx = last_visible_item.idx;
                    log::info!(
                        "popping excess ends after {} and line {}",
                        idx,
                        last_visible_item.line
                    );

                    while let Some(item) = previous_lines.pop_back() {
                        self.page_view.end = idx;
                        if item.idx <= idx {
                            previous_lines.push_back(item);
                            break;
                        }
                    }
                }

                self.lines_being_drawn.view = (visible_end.saturating_sub(height))..visible_end;
            }
            InstructionQueue::Resize { width, height } => {
                let is_width_changed = width != self.width;
                self.width = width;
                self.height = height;

                let mut visible_lines = previous_lines;
                // std::mem::replace(&mut self.lines_being_drawn.lines, LinkedList::new());
                if is_width_changed {
                    visible_lines.clear();

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
                }
                self.lines_being_drawn.view =
                    (visible_lines.len().saturating_sub(height))..visible_lines.len();

                self.requires_redraw = true;
            }
            InstructionQueue::JumpTo(idx) => {
                todo!()
            }
        };
    }

    pub fn view(&self) -> impl Iterator<Item = &LineWithIdx> {
        self.lines_being_drawn
            .lines
            .iter()
            .skip(self.lines_being_drawn.view.start)
            .take(self.lines_being_drawn.view.len())
    }
}

pub struct PageScrollWidget<'a> {
    state: &'a mut PageScrollState,
}

impl<'a> PageScrollWidget<'a> {
    pub fn new(state: &'a mut PageScrollState) -> Self {
        Self { state }
    }
}
impl<'a> Widget for PageScrollWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let padding = if self.state.show_line_numbers { 6 } else { 0 };
        let current_width = (area.width as usize - padding);
        let current_height = area.height as usize;
        if (self.state.width != current_width || self.state.height != current_height) {
            self.state.apply_queue(InstructionQueue::Resize {
                width: current_width,
                height: current_height,
            });
        } else {
            self.state.apply_queue(InstructionQueue::None);
        }

        if self.state.show_line_numbers {
            // move this allocation into ::resize, why keep track of them separetely, if it requires recomputation anyway

            let mut last_idx = 0;
            for (y, line) in self.state.view().enumerate() {
                if last_idx == line.idx {
                    // let s = format!("{:>6} {}", ' ', line.line);
                    let s = &line.line;
                    buf.set_string(padding as u16, area.y + y as u16, &s, Style::new());
                } else {
                    // let s = format!("{:>5} {}", line.idx, line.line);
                    buf.set_string(0, area.y + y as u16, &line.idx.to_string(), Style::new());
                    buf.set_string(padding as u16, area.y + y as u16, &line.line, Style::new());
                };
                last_idx = line.idx;
            }
        } else {
            for (y, line) in self.state.view().enumerate() {
                buf.set_string(0, area.y + y as u16, &line.line, Style::new());
            }
        }
    }
}

#[test]
fn test_new_scroll() {
    use std::fmt::Write;
    let mut state = PageScrollState::default();
    state.auto_scroll = true;
    let mut buf = String::with_capacity(512);

    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ    "
        .chars()
        .collect::<Vec<_>>();

    let mut rng = rand::rngs::StdRng::seed_from_u64(1);

    for i in 0..16 {
        // let len = 8 + rand::random::<usize>() % 32;
        let len = 8 + rng.gen::<usize>() % 24;
        buf.clear();

        for _ in 0..len {
            write!(&mut buf, "{} ", i);
        }

        state.page.add_line(&buf);
    }

    for (idx, line) in PageLineIterator::new(&state.page).enumerate() {
        print!("{}: '{}'\n", idx, line);
    }

    println!("^^^^^ current lines ^^^^^");

    state.apply_queue(InstructionQueue::Resize {
        width: 20,
        height: 10,
    });

    state.apply_queue(InstructionQueue::None);
    if (state.requires_redraw) {
        display(&state);
    }

    state.auto_scroll = false;

    println!("Going down");
    loop {
        state.apply_queue(InstructionQueue::Down);
        if (state.requires_redraw) {
            display(&state);
        } else {
            println!("Required no redraw");
            break;
        }
    }

    println!("Going Up");
    for i in 0..5 {
        state.apply_queue(InstructionQueue::Up);
        if (state.requires_redraw) {
            display(&state);
        }
    }

    state.auto_scroll = true;
    println!("Autoscroll");

    state.apply_queue(InstructionQueue::None);
    if (state.requires_redraw) {
        display(&state);
    }

    panic!("cuz rust don't show stdout");
}

fn display(state: &PageScrollState) {
    print!(
        "--------terminal (autoscroll:{}, size:{}x{})-------- real length {}, view {:?}, page_view: {:?}\n",
        state.auto_scroll,
        state.width,
        state.height,
        state.lines_being_drawn.lines.len(),
        state.lines_being_drawn.view,
        state.page_view,
    );
    println!("------------------------------------------------------------------------------------------------");
    let mut printed_start = false;
    let mut printed_end = false;
    for (idx, line) in state.lines_being_drawn.lines.iter().enumerate() {
        if state.lines_being_drawn.view.start == idx && !printed_start {
            print!("-----------start-------------\n");
            printed_start = true;
        }
        if state.lines_being_drawn.view.end == idx && !printed_end {
            print!("-----------end-------------\n");
            printed_end = true;
        }
        print!("{:>4}: {}\n", line.idx, line.line);
    }
    println!("------------------------------------------------------------------------------------------------");
}

pub fn main2() {
    // use crossterm::terminal;
    // let mut stdout = std::io::stdout();
    // terminal::enable_raw_mode().unwrap();
    // stdout.queue(terminal::EnterAlternateScreen).unwrap();
    // let _ = new_scroll_page();

    // stdout.queue(terminal::LeaveAlternateScreen).unwrap();
    // terminal::disable_raw_mode().unwrap();
}

use std::{
    collections::LinkedList,
    io::Write,
    ops::{Index, Range},
    slice::SliceIndex,
};

use crossterm::QueueableCommand;
use rand::{Rng, SeedableRng};
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
    /// returns if the instruction requires redraw    
    pub fn apply_queue(&mut self, queue: InstructionQueue) -> bool {
        let mut requires_redraw = false;
        let (width, height) = (self.width, self.height);

        let mut previous_lines = &mut self.lines_being_drawn.lines;
        // std::mem::replace(&mut self.lines_being_drawn.lines, LinkedList::new());

        let previous_start = previous_lines
            .front()
            .and_then(|x| Some(x.idx))
            .unwrap_or(0);
        let previous_end = previous_lines
            .back()
            .and_then(|x| Some(x.idx))
            .unwrap_or(self.page_view.end);

        // if self.auto_scroll {
        //     if self.page_view.end != self.page.len() {
        //         self.page_view.end = self.page.len();
        //         requires_redraw = true;
        //     }
        // }
        match queue {
            InstructionQueue::None => {
                if self.auto_scroll {
                    if self.page_view.end != self.page.len() {
                        self.page_view.end = self.page.len();

                        // todo auto scroll
                    } else {
                        self.lines_being_drawn.view =
                            (self.lines_being_drawn.lines.len().saturating_sub(height))
                                ..(self.lines_being_drawn.lines.len());
                    }
                }
            }
            InstructionQueue::Up => {
                if self.lines_being_drawn.view.end < previous_lines.len() {
                    self.lines_being_drawn.view.start += 1;
                    self.lines_being_drawn.view.end += 1;
                    requires_redraw = true;
                    return requires_redraw;
                }
                if self.page.len() > self.page_view.end {
                    // self.page_view.start += 1;
                    self.page_view.end += 1;
                    // self.get_lines_to_render(false);
                    requires_redraw = true;
                }

                /*
                     0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8
                old_start-^      old_end-^         ^-new_end

                  new_end > old_end

                  calculate old_end to new_end in reverse
                  append  (old_start to old_end) to  (old_end to new_end)

                 */
                let mut visible_lines = LinkedList::new();

                println!(
                    "pushing from {} to {} lines until filling height",
                    previous_end + 1,
                    self.page_view.end
                );
                for i in (previous_end + 1..self.page_view.end + 1).into_iter().rev() {
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
                println!("putting previous lines infront of visible lines");
                previous_lines.append(&mut visible_lines);

                let mut visible_start = visible_end.saturating_sub(height);
                let start_idx = previous_lines
                    .iter()
                    .nth(visible_start)
                    .and_then(|x| Some(x.idx))
                    .unwrap_or(0);
                println!("popping front lines with idx < {start_idx}");

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
                // self.lines_being_drawn = LinesToRenderAndView {
                //     view: ,
                //     lines: previous_lines,
                // };
            }
            InstructionQueue::Down => {
                if self.lines_being_drawn.view.start > 0 {
                    self.lines_being_drawn.view.start -= 1;
                    self.lines_being_drawn.view.end -= 1;
                    requires_redraw = true;
                    return requires_redraw;
                }
                if self.page_view.start > 0 {
                    // self.page_view.end -= 1;
                    self.page_view.start -= 1;
                    // self.get_lines_to_render(false);
                    requires_redraw = true;
                } else {
                    return requires_redraw;
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

                assert!(previous_start == self.page_view.start + 1);

                println!("pushing front lines until filling height");
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
                    println!(
                        "popping excess ends after {} and line {}",
                        idx, last_visible_item.line
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

                requires_redraw = true;
            }
        };

        return requires_redraw;
    }

    // pub fn apply_queue(&mut self, queue: InstructionQueue) -> bool {
    //     let mut requires_redraw = false;
    //     let mut requires_getting_lines = false;
    //     let mut is_width_changed = false;
    //     if self.auto_scroll {
    //         if self.page_view.end != self.page.len() {
    //             self.page_view.end = self.page.len();
    //             requires_getting_lines = true;
    //             requires_redraw = true;
    //         }
    //     }

    //     match queue {
    //         InstructionQueue::None => {
    //             // if requires_redraw {
    //             //     self.get_lines_to_render(false);
    //             // }
    //         }
    //         InstructionQueue::Up => {
    //             // if self.lines_being_drawn.view.end < self.lines_being_drawn.lines.len() {
    //             //     self.lines_being_drawn.view.start += 1;
    //             //     self.lines_being_drawn.view.end += 1;
    //             //     requires_redraw = true;
    //             // } else {
    //             //     if self.page.len() > self.page_view.end {
    //             //         // self.page_view.start += 1;
    //             //         self.page_view.end += 1;
    //             //         // self.get_lines_to_render(false);
    //             //         requires_redraw = true;
    //             //         requires_getting_lines = true;
    //             //     }
    //             // }
    //         }
    //         InstructionQueue::Down => {
    //             // if self.lines_being_drawn.view.start > 0 {
    //             //     self.lines_being_drawn.view.start -= 1;
    //             //     self.lines_being_drawn.view.end -= 1;
    //             //     requires_redraw = true;
    //             // } else {
    //             //     if self.page_view.start > 0 && self.page_view.end > 0 {
    //             //         // self.page_view.end -= 1;
    //             //         self.page_view.start -= 1;
    //             //         // self.get_lines_to_render(false);
    //             //         requires_redraw = true;
    //             //         requires_getting_lines = true;
    //             //     }
    //             // }
    //         }
    //         InstructionQueue::Resize { width, height } => {
    //             is_width_changed = self.width != width;
    //             self.width = width;
    //             self.height = height;
    //             // self.get_lines_to_render(is_width_changed);
    //             requires_getting_lines = true;
    //             requires_redraw = true;
    //         }
    //     }

    //     if requires_getting_lines {
    //         // self.get_lines_to_render(is_width_changed);
    //     }

    //     requires_redraw
    // }

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

    if state.apply_queue(InstructionQueue::Resize {
        width: 20,
        height: 10,
    }) {
        display(&state);
    }

    state.auto_scroll = false;

    println!("Going down");
    loop {
        if state.apply_queue(InstructionQueue::Down) {
            display(&state);
        } else {
            println!("Required no redraw");
            break;
        }
    }

    println!("Going Up");
    for i in 0..5 {
        if state.apply_queue(InstructionQueue::Up) {
            display(&state);
        }
    }

    state.auto_scroll = true;
    println!("Autoscroll");
    if state.apply_queue(InstructionQueue::None) {
        display(&state);
    }

    panic!();
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

// fn new_scroll_page() -> anyhow::Result<()> {
//     use crossterm::cursor;
//     use crossterm::terminal;

//     use std::fmt::Write;
//     use std::time::Duration;
//     let mut stdout = std::io::stdout();
//     let mut state = PageScrollState::default();

//     let mut rng = rand::rngs::StdRng::seed_from_u64(1);

//     let mut buf = String::new();
//     for i in 0..16 {
//         // let len = 8 + rand::random::<usize>() % 32;
//         let len = 8 + rng.gen::<usize>() % 32;
//         buf.clear();

//         for _ in 0..len {
//             write!(&mut buf, "{} ", i);
//         }

//         state.page.add_line(&buf);
//     }

//     state.auto_scroll = true;
//     state.apply_queue(InstructionQueue::Resize {
//         width: 20,
//         height: 10,
//     });

//     while let Ok(has_event) = crossterm::event::poll(Duration::from_millis(60)) {
//         if !has_event {
//             continue;
//         }
//         let event = crossterm::event::read()?;
//         let mut queue = InstructionQueue::None;
//         match event {
//             crossterm::event::Event::Key(key) => {
//                 match key.code {
//                     crossterm::event::KeyCode::Char(c) => match c {
//                         'j' => {
//                             queue = InstructionQueue::Down;
//                         }
//                         'k' => {
//                             queue = InstructionQueue::Up;
//                         }
//                         'a' => {
//                             state.auto_scroll = !state.auto_scroll;
//                         }
//                         'q' => {
//                             break;
//                         }
//                         _ => {}
//                     },
//                     _ => {}
//                 };
//             }
//             crossterm::event::Event::Resize(_, _) => {}
//             _ => {}
//         }

//         if state.apply_queue(queue) {
//             stdout.queue(terminal::Clear(terminal::ClearType::All))?;
//             stdout.queue(cursor::MoveTo(0, 0))?;
//             let mut y = 0;

//             for item in state.lines_being_drawn.lines.iter()
//             // .skip(state.lines_being_drawn.view.start)
//             // .take(state.lines_being_drawn.view.len())
//             {
//                 if state.lines_being_drawn.view.start == item.idx {
//                     stdout.queue(cursor::MoveTo(0, y as u16))?;
//                     y += 1;
//                     write!(stdout, "-----------start-------------");
//                 }
//                 if state.lines_being_drawn.view.end == item.idx {
//                     stdout.queue(cursor::MoveTo(0, y as u16))?;
//                     y += 1;
//                     write!(stdout, "-----------end-------------");
//                 }
//                 stdout.queue(cursor::MoveTo(0, y as u16))?;
//                 write!(stdout, "{:>5}: {}", item.idx, item.line)?;
//                 y += 1;
//             }
//         }

//         stdout.flush()?;
//     }

//     Ok(())
// }

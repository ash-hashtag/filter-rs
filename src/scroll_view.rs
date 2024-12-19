use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

use crate::{
    pages::{Page, PageSearchIterator},
    TuiMode,
};

#[derive(Default)]
pub struct ScrollState {
    scroll_position: usize,
    pub auto_scroll: bool,
}

impl ScrollState {
    pub fn go_up(&mut self) {
        self.scroll_position += 1;
        self.set_auto_scroll(false);
    }

    pub fn go_down(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }

        self.set_auto_scroll(false);
    }

    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
    }

    pub fn set_max_scroll_offset(&mut self) {
        self.scroll_position = 10000000;
    }
}

pub struct ScrollView<'a> {
    app_state: &'a mut AppState,
    scroll_state: &'a mut ScrollState,
}

impl<'a> ScrollView<'a> {
    pub fn new(state: &'a mut AppState, scroll_state: &'a mut ScrollState) -> Self {
        Self {
            app_state: state,
            scroll_state,
        }
    }

    pub fn render_wrapped_lines(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let height = area.height;
        let width = area.width;
        let mut vertical_position = self.scroll_state.scroll_position;

        let lines = &self.app_state.page;

        if self.scroll_state.auto_scroll {
            vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        }

        // log::info!("number of lines {}", lines.len());

        let start = (vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());
        let mut visible_lines = Vec::<String>::new();

        if self.app_state.show_line_numbers {
            for idx in (start..end).into_iter().rev() {
                let current_line = &lines[idx];
                let padding = 6;
                let wrapped_lines = textwrap::wrap(&current_line, width as usize - padding);
                if wrapped_lines.len() + visible_lines.len() <= height as usize {
                    let line_number = idx;
                    for l in wrapped_lines.iter().rev() {
                        let f_line = format!("{:>5} {}", line_number, l);
                        visible_lines.push(f_line);
                    }
                } else {
                    break;
                }
            }
        } else {
            for idx in (start..end).into_iter().rev() {
                let current_line = &lines[idx];

                let wrapped_lines = textwrap::wrap(&current_line, width as usize);
                if wrapped_lines.len() + visible_lines.len() <= height as usize {
                    for l in wrapped_lines.iter().rev() {
                        visible_lines.push(l.to_string());
                    }
                } else {
                    break;
                }
            }
        }

        // REDUNDANT REVERSE, AVOID IT
        visible_lines.reverse();
        for (y, line) in visible_lines.iter().enumerate() {
            buffer.set_string(0, area.y + y as u16, line, Style::new());
        }

        if end == lines.len() {
            self.scroll_state.set_auto_scroll(true);
        }

        self.scroll_state.scroll_position = vertical_position;
    }

    pub fn render_searched_lines(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let height = area.height;
        let width = area.width;
        let search_str = self.app_state.search_str();
        let mut vertical_position = self.scroll_state.scroll_position;

        log::info!("Searching for {}", search_str);

        let lines = PageSearchIterator::new(&self.app_state.page, search_str).collect::<Vec<_>>();

        if self.scroll_state.auto_scroll {
            vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        }
        let start = (vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());

        let padding = 6;

        struct LineToDraw {
            s: String,
            substr_start: isize,
        }
        let mut visible_lines = Vec::<LineToDraw>::new();

        if self.app_state.show_line_numbers {
            for idx in (start..end).into_iter().rev() {
                let line = &lines[idx];
                let wrapped_lines = textwrap::wrap(&line.line, width as usize - padding);
                if wrapped_lines.len() + visible_lines.len() <= height as usize {
                    let line_number = line.line_index;
                    let mut cursor = line.line.len();
                    for l in wrapped_lines.iter().rev() {
                        let f_line = format!("{:>5} {}", line_number, l);
                        let mut substr_start = -1isize;

                        if line.substr_start < cursor && line.substr_start > cursor - l.len() {
                            substr_start =
                                (padding + (line.substr_start - (cursor - l.len()))) as isize;
                        }

                        visible_lines.push(LineToDraw {
                            s: f_line,
                            substr_start,
                        });

                        cursor -= l.len();
                    }
                } else {
                    break;
                }
            }
        } else {
            for idx in (start..end).into_iter().rev() {
                let line = &lines[idx];
                let wrapped_lines = textwrap::wrap(&line.line, width as usize);
                if wrapped_lines.len() + visible_lines.len() <= height as usize {
                    // let line_number = lines[idx].line_index;

                    /*

                    abcabcabcdddabcabc // 18
                    ---------^ 9

                    cursor = 18

                    bc       // 2 cursor = 18 -> 16


                    cdddabca // 8 cursor = 16 -> 8
                    abcabcab // 8 cursor = 8  -> 0

                    */

                    let mut cursor = line.line.len();
                    for l in wrapped_lines.iter().rev() {
                        let mut substr_start = -1isize;

                        if line.substr_start < cursor && line.substr_start > cursor - l.len() {
                            substr_start = (line.substr_start - (cursor - l.len())) as isize;
                        }

                        visible_lines.push(LineToDraw {
                            s: l.to_string(),
                            substr_start,
                        });
                        cursor -= l.len();
                    }
                } else {
                    break;
                }
            }
        }

        // REDUNDANT REVERSE, AVOID IT
        visible_lines.reverse();
        for (y, line_to_draw) in visible_lines.iter().enumerate() {
            let index = line_to_draw.substr_start;
            let line = &line_to_draw.s;
            if index >= 0 {
                if index as usize + search_str.len() <= line.len() {
                    let index = index as usize;
                    let prefix = &line[0..index];
                    let span = Span::raw(prefix);
                    let mut cursor = 0;
                    buffer.set_span(cursor, area.y + y as u16, &span, width);
                    cursor += span.width() as u16;
                    let span = Span::raw(&line[index..index + search_str.len()])
                        .bg(ratatui::style::Color::Yellow)
                        .fg(ratatui::style::Color::Black);
                    buffer.set_span(cursor, area.y + y as u16, &span, width);
                    cursor += span.width() as u16;
                    let suffix = &line[index + search_str.len()..];
                    let span = Span::raw(suffix);
                    buffer.set_span(cursor, area.y + y as u16, &span, width);
                } else {
                    log::warn!("Invalid index {index} of {line} search_str: {search_str}");

                    buffer.set_string(0, area.y + y as u16, line, Style::new());
                }
            } else {
                buffer.set_string(0, area.y + y as u16, line, Style::new());
            }
        }

        if end == lines.len() {
            self.scroll_state.set_auto_scroll(true);
        }

        self.scroll_state.scroll_position = vertical_position;
    }
}

impl<'a> Widget for ScrollView<'a> {
    fn render(mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer)
    where
        Self: Sized,
    {
        if self.app_state.is_in_search_mode() {
            self.render_searched_lines(area, buffer);
        } else {
            self.render_wrapped_lines(area, buffer);
        }
    }
}

pub struct AppState {
    pub mode: TuiMode,
    pub command: String,
    pub page: Page,
    pub show_line_numbers: bool,
}

impl AppState {
    pub fn new(page: Page, mode: TuiMode) -> Self {
        Self {
            mode,
            command: String::new(),
            page,
            show_line_numbers: false,
        }
    }

    pub fn set_mode(&mut self, mode: TuiMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> TuiMode {
        self.mode
    }

    pub fn add_line(&mut self, s: &str) {
        self.page.add_line(s);
    }
    pub fn is_in_search_mode(&self) -> bool {
        self.command.len() > 1 && self.command.starts_with("/")
    }

    pub fn search_str(&self) -> &str {
        if self.is_in_search_mode() {
            &self.command[1..]
        } else {
            ""
        }
    }
}

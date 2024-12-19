use ratatui::{buffer::Buffer, style::Style, widgets::Widget};

use crate::{
    pages::{Page, PageSearchIterator},
    TuiMode,
};

pub struct ScrollView<'a> {
    state: &'a mut ScrollViewState,
}

impl<'a> ScrollView<'a> {
    pub fn new(state: &'a mut ScrollViewState) -> Self {
        Self { state }
    }

    pub fn render_wrapped_lines(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let height = area.height;
        let width = area.width;
        let mut vertical_position = self.state.vertical_position;

        let lines = &self.state.page;

        if self.state.auto_scroll {
            vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        }
        let start = (vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());
        let mut visible_lines = Vec::new();
        let padding = 6;

        for idx in (start..end).into_iter().rev() {
            let wrapped_lines = textwrap::wrap(&lines[idx], width as usize - padding);
            if wrapped_lines.len() + visible_lines.len() <= height as usize {
                // visible_lines.extend_from_slice(&lines);
                let line_number = idx;
                for l in wrapped_lines.iter().rev() {
                    let f_line = format!("{:>5} {}", line_number, l);

                    visible_lines.push(f_line);
                }
            } else {
                break;
            }
        }

        visible_lines.reverse();
        for (y, line) in visible_lines.iter().enumerate() {
            buffer.set_string(0, area.y + y as u16, line, Style::new());
        }

        if end == self.state.page.len() {
            self.state.set_auto_scroll(true);
        }

        self.state.vertical_position = vertical_position;
    }

    pub fn render_searched_lines(&mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer) {
        let search_str = &self.state.search_str();

        log::info!("Searching for {search_str}",);

        let iterator = PageSearchIterator::new(&self.state.page, search_str).rev();
        let lines = iterator.collect::<Vec<_>>();

        // log::info!("Found lines with {search_str} {:?}", lines);

        let height = area.height;
        let width = area.width;
        let mut vertical_position = self.state.search_vertical_position;

        if self.state.auto_scroll {
            vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        }
        let start = (vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());

        let mut visible_lines = Vec::new();
        let padding = 6;
        for idx in (start..end).into_iter().rev() {
            let wrapped_lines = textwrap::wrap(lines[idx].line, width as usize - padding);
            if wrapped_lines.len() + visible_lines.len() <= height as usize {
                // visible_lines.extend_from_slice(&lines);
                let line_number = lines[idx].line_index;
                for l in wrapped_lines.iter().rev() {
                    let f_line = format!("{:>5} {}", line_number, l);

                    visible_lines.push(f_line);
                }
            } else {
                break;
            }
        }

        // visible_lines.reverse();

        for (y, line) in visible_lines.iter().enumerate() {
            buffer.set_string(0, area.y + y as u16, line, Style::new());
        }

        if end == self.state.page.len() {
            self.state.set_auto_scroll(true);
        }

        self.state.search_vertical_position = vertical_position;
    }
}

impl<'a> Widget for ScrollView<'a> {
    fn render(mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer)
    where
        Self: Sized,
    {
        if self.state.is_in_search_mode() {
            self.render_searched_lines(area, buffer);
        } else {
            self.render_wrapped_lines(area, buffer);
        }
    }
}

pub struct ScrollViewState {
    vertical_position: usize,
    search_vertical_position: usize,
    pub mode: TuiMode,
    pub auto_scroll: bool,
    pub command: String,
    pub page: Page,
    pub page_view: std::ops::Range<usize>,
}

impl ScrollViewState {
    pub fn new(page: Page, mode: TuiMode, auto_scroll: bool) -> Self {
        Self {
            vertical_position: 0,
            search_vertical_position: 0,
            mode,
            auto_scroll,
            command: String::new(),
            page,
            page_view: 0..0,
        }
    }

    pub fn set_mode(&mut self, mode: TuiMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> TuiMode {
        self.mode
    }

    // pub fn add_content(&mut self, s: &str) {
    //     self.content += s;
    // }

    pub fn add_line(&mut self, s: &str) {
        self.page.add_line(s);
    }

    // pub fn get_content(&self) -> &str {
    //     &self.content
    // }

    pub fn go_up(&mut self) {
        if self.is_in_search_mode() {
            self.search_vertical_position += 1;
        } else {
            self.vertical_position += 1;
        }
        self.set_auto_scroll(false);
    }

    pub fn go_down(&mut self) {
        if self.is_in_search_mode() {
            if self.vertical_position > 0 {
                self.vertical_position -= 1;
            }
        } else {
            if self.vertical_position > 0 {
                self.vertical_position -= 1;
            }
        }

        self.set_auto_scroll(false);
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

    // pub fn get_auto_scroll(&mut self) -> bool {
    //     self.auto_scroll
    // }
    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
    }
}

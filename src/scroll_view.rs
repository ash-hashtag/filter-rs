use ratatui::{buffer::Buffer, style::Style, widgets::Widget};

use crate::{pages::Page, TuiMode};

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

        if self.state.auto_scroll {
            vertical_position = self
                .state
                .page
                .len()
                .checked_sub(height as usize)
                .unwrap_or(0);
        }
        let start = (vertical_position).min(self.state.page.len());
        let end = (start + height as usize).min(self.state.page.len());

        let last_viewed_lines = self.state.page.get_slice(start..end).unwrap();
        let mut visible_lines = Vec::new();
        let padding = 6;

        for (idx, line) in last_viewed_lines.iter().rev().enumerate() {
            let lines = textwrap::wrap(line, width as usize - padding);
            if lines.len() + visible_lines.len() <= height as usize {
                // visible_lines.extend_from_slice(&lines);
                let line_number = end - idx;
                for l in lines.iter().rev() {
                    visible_lines.push(format!("{:>5} {}", line_number, l));
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
}

impl<'a> Widget for ScrollView<'a> {
    fn render(mut self, area: ratatui::prelude::Rect, buffer: &mut Buffer)
    where
        Self: Sized,
    {
        // let height = area.height;
        // let width = area.width;

        // let lines = textwrap::wrap(self.state.get_content(), width as usize);
        // let mut vertical_position = self.state.vertical_position;

        // if self.state.auto_scroll {
        //     vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        // }
        // let start = (vertical_position).min(lines.len());
        // let end = (start + height as usize).min(lines.len());

        // let visible_lines = &lines[start..end];

        // for (y, line) in visible_lines.iter().enumerate() {
        //     let f_line = format!("{:>5} {}", start + y, line,);

        //     buffer.set_string(0, area.y + y as u16, &f_line, Style::new());
        // }

        // if end == lines.len() {
        //     self.state.set_auto_scroll(true);
        // }
        // self.state.vertical_position = vertical_position;
        self.render_wrapped_lines(area, buffer);
    }
}

pub struct ScrollViewState {
    vertical_position: usize,
    content: String,
    pub mode: TuiMode,
    pub auto_scroll: bool,
    pub command: String,
    pub page: Page,
    pub page_view: std::ops::Range<usize>,
}

impl ScrollViewState {
    pub fn new(
        vertical_position: usize,
        content: String,
        mode: TuiMode,
        auto_scroll: bool,
    ) -> Self {
        Self {
            vertical_position,
            content,
            mode,
            auto_scroll,
            command: String::new(),
            page: Page::new(),
            page_view: 0..0,
        }
    }

    pub fn set_mode(&mut self, mode: TuiMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> TuiMode {
        self.mode
    }

    pub fn add_content(&mut self, s: &str) {
        self.content += s;
    }

    pub fn add_line(&mut self, s: &str) {
        self.page.add_line(s);
    }

    // pub fn get_content(&self) -> &str {
    //     &self.content
    // }

    pub fn go_up(&mut self) {
        self.vertical_position += 1;
        self.set_auto_scroll(false);
    }

    pub fn go_down(&mut self) {
        if self.vertical_position > 0 {
            self.vertical_position -= 1;
        }
        self.set_auto_scroll(false);
    }

    // pub fn get_auto_scroll(&mut self) -> bool {
    //     self.auto_scroll
    // }
    pub fn set_auto_scroll(&mut self, auto_scroll: bool) {
        self.auto_scroll = auto_scroll;
    }
}

pub struct SearchResultLine<'a> {
    line: &'a str,
    substr_start: usize,
    substr_end: usize,
}

fn get_searched_lines<'a, 'b>(
    main_content: &'a str,
    search_str: &'b str,
) -> Vec<SearchResultLine<'a>> {
    let mut lines = Vec::new();
    for line in main_content.lines() {
        if let Some(index) = line.find(search_str) {
            lines.push(SearchResultLine {
                line,
                substr_start: index,
                substr_end: index + search_str.len(),
            });
        }
    }

    lines
}

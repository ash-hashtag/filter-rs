use ratatui::{buffer::Buffer, style::Style, widgets::Widget};
use std::ops::Range;

use crate::command::Matcher;
use crate::pages::Pages;
use std::sync::{Arc, RwLock};

pub fn get_wrapped_lines(s: &str, width: usize) -> Vec<(Box<str>, Range<usize>)> {
    let options = textwrap::Options::new(width);
    textwrap::wrap(s, &options)
        .iter()
        .map(|x| {
            let start = x.as_ptr() as usize - s.as_ptr() as usize;
            (x.as_ref().into(), start..start + x.len())
        })
        .collect()
}

pub struct PageScrollState {
    pages: Arc<RwLock<Pages>>,
    show_line_numbers: bool,
    auto_scroll: bool,
    width: usize,
    height: usize,

    // Manual scroll state
    bottom_line_idx: usize,
    bottom_line_wrapped_skip: usize, // number of sub-lines of bottom_line_idx to skip from the bottom

    // Highlighted cursor
    cursor_idx: Option<usize>,
    cursor_range: Option<Range<usize>>,

    // Filter
    filter: Option<crate::command::Command>,
    // Search highlight
    pub search_query: Option<crate::command::Command>,
}

impl PageScrollState {
    pub fn new(pages: Arc<RwLock<Pages>>) -> Self {
        Self {
            pages,
            show_line_numbers: false,
            auto_scroll: true,
            width: 0,
            height: 0,
            bottom_line_idx: 0,
            bottom_line_wrapped_skip: 0,
            cursor_idx: None,
            cursor_range: None,
            filter: None,
            search_query: None,
        }
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
    }

    pub fn toggle_autoscroll(&mut self) {
        if self.auto_scroll {
            let pages_len = self.pages.read().unwrap().lines_count();
            self.bottom_line_idx = pages_len.saturating_sub(1);
            self.bottom_line_wrapped_skip = 0;
        }
        self.auto_scroll = !self.auto_scroll;
    }

    pub fn auto_scroll(&self) -> bool {
        self.auto_scroll
    }

    pub fn show_line_numbers(&self) -> bool {
        self.show_line_numbers
    }

    pub fn scroll_up(&mut self) {
        let pages_len = self.pages.read().unwrap().lines_count();
        if pages_len == 0 {
            return;
        }

        if self.auto_scroll {
            self.auto_scroll = false;
            self.bottom_line_idx = pages_len.saturating_sub(1);
            self.bottom_line_wrapped_skip = 0;
            // After disabling autoscroll, we proceed to perform the actual scroll up.
        }

        let padding = if self.show_line_numbers { 6 } else { 0 };
        let render_width = self.width.saturating_sub(padding);
        log::debug!("scroll_up: pages_len={}, width={}, render_width={}, auto_scroll={}, bottom_idx={}, skip={}", 
            pages_len, self.width, render_width, self.auto_scroll, self.bottom_line_idx, self.bottom_line_wrapped_skip);

        if render_width == 0 {
            return;
        }

        let pages = self.pages.read().unwrap();
        if let Some(line) = pages.get_line(self.bottom_line_idx) {
            let wrapped_count = get_wrapped_lines(line, render_width).len();
            if self.bottom_line_wrapped_skip + 1 < wrapped_count {
                self.bottom_line_wrapped_skip += 1;
            } else if self.bottom_line_idx > pages.first_index() {
                self.bottom_line_idx -= 1;
                self.bottom_line_wrapped_skip = 0;
            }
        }
    }

    pub fn scroll_down(&mut self) {
        let pages_len = self.pages.read().unwrap().lines_count();
        if pages_len == 0 {
            return;
        }

        if self.auto_scroll {
            return;
        }

        log::debug!(
            "scroll_down: pages_len={}, width={}, auto_scroll={}, bottom_idx={}, skip={}",
            pages_len,
            self.width,
            self.auto_scroll,
            self.bottom_line_idx,
            self.bottom_line_wrapped_skip
        );

        if self.bottom_line_wrapped_skip > 0 {
            self.bottom_line_wrapped_skip -= 1;
        } else {
            self.bottom_line_idx += 1;
            if self.bottom_line_idx >= pages_len {
                self.bottom_line_idx = pages_len.saturating_sub(1);
                self.auto_scroll = true;
            }
        }
    }

    pub fn jump_to(&mut self, idx: usize) {
        let pages_len = self.pages.read().unwrap().lines_count();
        if idx < pages_len {
            self.auto_scroll = false;
            self.bottom_line_idx = idx;
            self.bottom_line_wrapped_skip = 0;
            self.cursor_idx = Some(idx);
            self.cursor_range = None;
        }
    }

    pub fn jump_to_with_range(&mut self, idx: usize, range: Range<usize>) {
        let pages_len = self.pages.read().unwrap().lines_count();
        if idx < pages_len {
            self.auto_scroll = false;
            self.bottom_line_idx = idx;
            self.bottom_line_wrapped_skip = 0;
            self.cursor_idx = Some(idx);
            self.cursor_range = Some(range);
        }
    }

    pub fn set_cursor(&mut self, idx: Option<usize>) {
        self.cursor_idx = idx;
    }

    pub fn bottom_line_idx(&self) -> usize {
        self.bottom_line_idx
    }

    pub fn set_filter(&mut self, filter: Option<crate::command::Command>) {
        self.filter = filter;
    }

    pub fn filter(&self) -> Option<&crate::command::Command> {
        self.filter.as_ref()
    }

    pub fn set_search_query(&mut self, query: Option<crate::command::Command>) {
        self.search_query = query;
    }

    pub fn cursor_idx(&self) -> Option<usize> {
        self.cursor_idx
    }
}

pub struct PageScrollWidget<'a>(pub &'a PageScrollState);

impl<'a> Widget for PageScrollWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut Buffer) {
        let state = self.0;
        let pages = state.pages.read().unwrap();
        let pages_len = pages.lines_count();
        if pages_len == 0 {
            return;
        }

        let padding = if state.show_line_numbers { 6 } else { 0 };
        let render_width = (area.width as usize).saturating_sub(padding);
        if render_width == 0 {
            return;
        }

        let mut lines_to_render = Vec::new();
        let height = area.height as usize;

        // Start from the bottom anchor and work backwards
        let mut current_idx = if state.auto_scroll {
            pages_len.saturating_sub(1)
        } else {
            state.bottom_line_idx.min(pages_len.saturating_sub(1))
        };

        let mut skip_sublines = if state.auto_scroll {
            0
        } else {
            state.bottom_line_wrapped_skip
        };

        log::debug!(
            "render: pages_len={}, area={:?}, auto_scroll={}, bottom_idx={}, skip={}",
            pages_len,
            area,
            state.auto_scroll,
            state.bottom_line_idx,
            state.bottom_line_wrapped_skip
        );

        'outer: loop {
            if let Some(line_content) = pages.get_line(current_idx) {
                let mut highlight = None;
                if let Some(filter) = &state.filter {
                    if let Some(mat) = filter.is_match(line_content) {
                        highlight = Some(mat);
                    } else {
                        // Skip line if it doesn't match filter
                        if current_idx > pages.first_index() {
                            current_idx -= 1;
                            continue;
                        } else {
                            break;
                        }
                    }
                }

                // If no filter highlight, check if search_query matches
                if highlight.is_none() {
                    if let Some(search) = &state.search_query {
                        highlight = search.is_match(line_content);
                    }
                }

                let wrapped = get_wrapped_lines(line_content, render_width);
                for (w, source_range) in wrapped.into_iter().rev() {
                    if skip_sublines > 0 {
                        skip_sublines -= 1;
                        continue;
                    }
                    lines_to_render.push((current_idx, w, source_range, highlight.clone()));
                    if lines_to_render.len() >= height {
                        break 'outer;
                    }
                }
            }
            if current_idx <= pages.first_index() {
                break;
            }
            current_idx -= 1;
            skip_sublines = 0; // Only skip for the bottom-most log line
        }
        lines_to_render.reverse();

        for (i, (idx, line, source_range, filter_highlight)) in lines_to_render.iter().enumerate() {
            if i >= height {
                break;
            }

            let y = area.y + i as u16;
            let style = if Some(*idx) == state.cursor_idx && state.cursor_range.is_none() {
                Style::default().fg(ratatui::style::Color::Yellow)
            } else {
                Style::default()
            };

            if state.show_line_numbers {
                let line_num = format!("[{}]", idx);
                let num_padding = 5usize.saturating_sub(line_num.len());
                buf.set_string(area.x + num_padding as u16, y, &line_num, style);

                if let Some(range) = state.cursor_range.as_ref() {
                    if Some(*idx) == state.cursor_idx {
                        let green_style = Style::default()
                            .bg(ratatui::style::Color::Green)
                            .fg(ratatui::style::Color::Black);

                        self.render_line_partial(
                            buf,
                            area.x + padding as u16,
                            y,
                            line,
                            source_range,
                            range,
                            green_style,
                        );
                        continue;
                    }
                }

                if let Some(range) = filter_highlight {
                    let highlight_style = if Some(*idx) == state.cursor_idx {
                        Style::default()
                            .bg(ratatui::style::Color::Green)
                            .fg(ratatui::style::Color::Black)
                    } else {
                        Style::default()
                            .bg(ratatui::style::Color::Yellow)
                            .fg(ratatui::style::Color::Black)
                    };

                    self.render_line_partial(
                        buf,
                        area.x + padding as u16,
                        y,
                        line,
                        source_range,
                        range,
                        highlight_style,
                    );
                } else {
                    buf.set_string(area.x + padding as u16, y, line, style);
                }
            } else {
                if let Some(range) = state.cursor_range.as_ref() {
                    if Some(*idx) == state.cursor_idx {
                        let green_style = Style::default()
                            .bg(ratatui::style::Color::Green)
                            .fg(ratatui::style::Color::Black);
                        self.render_line_partial(
                            buf,
                            area.x,
                            y,
                            line,
                            source_range,
                            range,
                            green_style,
                        );
                        continue;
                    }
                }

                if let Some(range) = filter_highlight {
                    let highlight_style = if Some(*idx) == state.cursor_idx {
                        Style::default()
                            .bg(ratatui::style::Color::Green)
                            .fg(ratatui::style::Color::Black)
                    } else {
                        Style::default()
                            .bg(ratatui::style::Color::Yellow)
                            .fg(ratatui::style::Color::Black)
                    };
                    self.render_line_partial(
                        buf,
                        area.x,
                        y,
                        line,
                        source_range,
                        range,
                        highlight_style,
                    );
                } else {
                    buf.set_string(area.x, y, line, style);
                }
            }
        }
    }
}

impl<'a> PageScrollWidget<'a> {
    fn render_line_partial(
        &self,
        buf: &mut Buffer,
        x: u16,
        y: u16,
        segment_text: &str,
        segment_range: &Range<usize>,
        highlight_range: &Range<usize>,
        highlight_style: Style,
    ) {
        // Calculate the intersection of highlight_range and segment_range
        let intersect_start = highlight_range.start.max(segment_range.start);
        let intersect_end = highlight_range.end.min(segment_range.end);

        if intersect_start < intersect_end {
            // There is an intersection. Highlight only the intersected part.
            // Indices relative to the segment_text
            let rel_start = intersect_start - segment_range.start;
            let rel_end = intersect_end - segment_range.start;

            // Draw before highlight
            if rel_start > 0 {
                buf.set_string(x, y, &segment_text[..rel_start], Style::default());
            }

            // Draw highlight
            buf.set_string(
                x + rel_start as u16,
                y,
                &segment_text[rel_start..rel_end],
                highlight_style,
            );

            // Draw after highlight
            if rel_end < segment_text.len() {
                buf.set_string(
                    x + rel_end as u16,
                    y,
                    &segment_text[rel_end..],
                    Style::default(),
                );
            }
        } else {
            // No intersection
            buf.set_string(x, y, segment_text, Style::default());
        }
    }
}

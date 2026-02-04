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

    // Match tracking
    matches: Vec<usize>,
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
            matches: Vec::new(),
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
        let padding = if self.show_line_numbers { 6 } else { 0 };
        let render_width = self.width.saturating_sub(padding).max(1);
        if render_width == 0 {
            return;
        }

        let pages_arc = self.pages.clone();
        let pages_read = pages_arc.read().unwrap();
        let pages_len = pages_read.lines_count();
        if pages_len == 0 {
            return;
        }

        if self.auto_scroll {
            self.auto_scroll = false;
            self.bottom_line_idx = pages_len.saturating_sub(1);
            self.bottom_line_wrapped_skip = 0;
        }

        // Before scrolling up, check if we've already reached the top of the viewport
        if is_top_reached_helper(
            self.filter.as_ref(),
            self.show_line_numbers,
            self.width,
            self.height,
            self.auto_scroll,
            self.bottom_line_idx,
            self.bottom_line_wrapped_skip,
            &*pages_read,
        ) {
            return;
        }

        if let Some(line) = pages_read.get_line(self.bottom_line_idx) {
            let wrapped_count = get_wrapped_lines(line, render_width).len();
            if self.bottom_line_wrapped_skip + 1 < wrapped_count {
                self.bottom_line_wrapped_skip += 1;
            } else {
                // Find previous line that satisfies the filter (if any)
                let first_index = pages_read.first_index();
                let skip_from_back = pages_len.saturating_sub(self.bottom_line_idx);

                let mut it = pages_read.iter();
                it.fast_skip_back(skip_from_back);
                for (i, line) in it.enumerate().rev() {
                    if self
                        .filter
                        .as_ref()
                        .map_or(true, |f| f.is_match(line).is_some())
                    {
                        self.bottom_line_idx = first_index + i;
                        self.bottom_line_wrapped_skip = 0;
                        break;
                    }
                }
            }
        }
        drop(pages_read);
        self.normalize_scroll();
    }

    pub fn scroll_down(&mut self) {
        let pages_arc = self.pages.clone();
        let pages_read = pages_arc.read().unwrap();
        let pages_len = pages_read.lines_count();
        if pages_len == 0 {
            return;
        }

        if self.auto_scroll {
            return;
        }

        if self.bottom_line_wrapped_skip > 0 {
            self.bottom_line_wrapped_skip -= 1;
        } else {
            let skip = self.bottom_line_idx + 1 - pages_read.first_index();
            let first_index = pages_read.first_index();

            let mut it = pages_read.iter();
            it.fast_skip(skip);
            for (i, line) in it.enumerate() {
                if self
                    .filter
                    .as_ref()
                    .map_or(true, |f| f.is_match(line).is_some())
                {
                    self.bottom_line_idx = first_index + skip + i;
                    break;
                }
            }

            // If we reached the end or couldn't find more matches
            if self.bottom_line_idx == pages_len.saturating_sub(1) {
                self.auto_scroll = true;
            }
        }
        drop(pages_read);
        self.normalize_scroll();
    }

    pub fn jump_to(&mut self, idx: usize) {
        let pages_arc = self.pages.clone();
        let pages_read = pages_arc.read().unwrap();
        let pages_len = pages_read.lines_count();
        if idx < pages_len {
            self.auto_scroll = false;
            if !self.is_idx_visible_internal(&*pages_read, idx) {
                self.bottom_line_idx = idx;
                self.bottom_line_wrapped_skip = 0;
            }
            self.cursor_idx = Some(idx);
            self.cursor_range = None;
            drop(pages_read);
            self.normalize_scroll();
        }
    }

    pub fn jump_to_with_range(&mut self, idx: usize, range: Range<usize>) {
        let pages_arc = self.pages.clone();
        let pages_read = pages_arc.read().unwrap();
        let pages_len = pages_read.lines_count();
        if idx < pages_len {
            self.auto_scroll = false;
            if !self.is_idx_visible_internal(&*pages_read, idx) {
                self.bottom_line_idx = idx;
                self.bottom_line_wrapped_skip = 0;
            }
            self.cursor_idx = Some(idx);
            self.cursor_range = Some(range);
            drop(pages_read);
            self.normalize_scroll();
        }
    }

    pub fn set_cursor(&mut self, idx: Option<usize>) {
        self.cursor_idx = idx;
        if idx.is_none() {
            self.cursor_range = None;
        }
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
        self.matches.clear();
    }

    pub fn set_matches(&mut self, matches: Vec<usize>) {
        self.matches = matches;
    }

    pub fn add_match(&mut self, idx: usize) {
        if !self.matches.contains(&idx) {
            self.matches.push(idx);
            self.matches.sort_unstable();
        }
    }

    pub fn remove_matches_before(&mut self, idx: usize) {
        self.matches.retain(|&m| m >= idx);
    }

    pub fn get_match_status(&self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }

        let total = self.matches.len();
        let current_pos = self.cursor_idx.or(Some(self.bottom_line_idx))?;

        // Find the rank of the current_pos among matches.
        // If current_pos is exactly a match, show its rank.
        // If not, maybe show the rank of the next match?
        // User said "our cursor's position in there", so let's find the index in self.matches.
        let rank = match self.matches.binary_search(&current_pos) {
            Ok(idx) => idx + 1,
            Err(idx) => {
                // Not exactly on a match. idx is where it would be inserted.
                // Let's show the one before it, or 1 if it's before all.
                if idx == 0 {
                    1
                } else {
                    idx
                }
            }
        };

        Some((rank, total))
    }

    pub fn cursor_idx(&self) -> Option<usize> {
        self.cursor_idx
    }

    fn is_idx_visible_internal(&self, pages: &Pages, target_idx: usize) -> bool {
        let padding = if self.show_line_numbers { 6 } else { 0 };
        let render_width = self.width.saturating_sub(padding).max(1);
        if self.height == 0 {
            return false;
        }

        let pages_len = pages.lines_count();
        if pages_len == 0 {
            return false;
        }

        let end_idx = if self.auto_scroll {
            pages_len.saturating_sub(1)
        } else {
            self.bottom_line_idx.min(pages_len.saturating_sub(1))
        };

        if target_idx > end_idx {
            return false;
        }

        let mut skip_sublines = if self.auto_scroll {
            0
        } else {
            self.bottom_line_wrapped_skip
        };
        let mut total_rendered_lines = 0;
        let skip_from_back = pages_len.saturating_sub(end_idx + 1);

        let mut it = pages.iter();
        it.fast_skip_back(skip_from_back);
        for (i, line_content) in it.enumerate().rev() {
            let current_idx = pages.first_index() + i;
            if self
                .filter
                .as_ref()
                .map_or(true, |f| f.is_match(line_content).is_some())
            {
                if current_idx == target_idx {
                    return true;
                }

                let wrapped_len = get_wrapped_lines(line_content, render_width).len();
                let effective_lines = wrapped_len.saturating_sub(skip_sublines);

                total_rendered_lines += effective_lines;

                if total_rendered_lines >= self.height {
                    return false; // Viewport is full and we didn't hit the target
                }
            }
            skip_sublines = 0;
        }

        false
    }

    pub fn normalize_scroll(&mut self) {
        if self.auto_scroll || self.height == 0 {
            return;
        }

        // Clone Arc to decouple borrow from self
        let pages_arc = self.pages.clone();
        let pages = pages_arc.read().unwrap();

        let mut current_bottom_idx = self.bottom_line_idx;
        let mut current_wrapped_skip = self.bottom_line_wrapped_skip;

        // While the top of the file is visible and there's potentially more to show at the bottom,
        // scroll down (increase bottom_line_idx) to fill the gap.
        while is_top_reached_helper(
            self.filter.as_ref(),
            self.show_line_numbers,
            self.width,
            self.height,
            self.auto_scroll,
            current_bottom_idx,
            current_wrapped_skip,
            &*pages,
        ) {
            let pages_len = pages.lines_count();
            if current_bottom_idx + 1 >= pages_len {
                break;
            }

            let first_index = pages.first_index();
            let skip = current_bottom_idx + 1 - first_index;
            let mut it = pages.iter();
            it.fast_skip(skip);

            let mut found = false;
            for (i, line) in it.enumerate() {
                if self
                    .filter
                    .as_ref()
                    .map_or(true, |f| f.is_match(line).is_some())
                {
                    current_bottom_idx = first_index + skip + i;
                    current_wrapped_skip = 0;
                    found = true;
                    break;
                }
            }

            if !found {
                break;
            }
        }

        self.bottom_line_idx = current_bottom_idx;
        self.bottom_line_wrapped_skip = current_wrapped_skip;
    }
}

fn is_top_reached_helper(
    filter: Option<&crate::command::Command>,
    show_line_numbers: bool,
    width: usize,
    height: usize,
    auto_scroll: bool,
    bottom_line_idx: usize,
    bottom_line_wrapped_skip: usize,
    pages: &Pages,
) -> bool {
    let padding = if show_line_numbers { 6 } else { 0 };
    let render_width = width.saturating_sub(padding).max(1);
    if height == 0 {
        return false;
    }

    let pages_len = pages.lines_count();
    if pages_len == 0 {
        return true;
    }

    let end_idx = if auto_scroll {
        pages_len.saturating_sub(1)
    } else {
        bottom_line_idx.min(pages_len.saturating_sub(1))
    };

    let mut skip_sublines = if auto_scroll {
        0
    } else {
        bottom_line_wrapped_skip
    };

    let mut total_rendered_lines = 0;
    let skip_from_back = pages_len.saturating_sub(end_idx + 1);

    let mut it = pages.iter();
    it.fast_skip_back(skip_from_back);
    for line_content in it.rev() {
        if filter.map_or(true, |f| f.is_match(line_content).is_some()) {
            let wrapped_len = get_wrapped_lines(line_content, render_width).len();
            let effective_lines = wrapped_len.saturating_sub(skip_sublines);

            total_rendered_lines += effective_lines;

            if total_rendered_lines >= height {
                return false; // Viewport is full
            }
        }
        skip_sublines = 0;
    }

    // If we've processed all matching lines and viewport is not full, the top is reached
    total_rendered_lines < height
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

        let height = area.height as usize;
        let mut lines_to_render = Vec::new();

        // Start from the bottom anchor and work backwards
        let end_idx = if state.auto_scroll {
            pages_len.saturating_sub(1)
        } else {
            state.bottom_line_idx.min(pages_len.saturating_sub(1))
        };

        let mut skip_sublines = if state.auto_scroll {
            0
        } else {
            state.bottom_line_wrapped_skip
        };

        let first_index = pages.first_index();
        let skip_from_back = pages_len.saturating_sub(end_idx + 1);

        let mut it = pages.iter();
        it.fast_skip_back(skip_from_back);
        'outer: for (i, line_content) in it.enumerate().rev() {
            let current_idx = first_index + i;
            let mut highlight = None;
            if let Some(filter) = &state.filter {
                if let Some(mat) = filter.is_match(line_content) {
                    highlight = Some(mat);
                } else {
                    continue;
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

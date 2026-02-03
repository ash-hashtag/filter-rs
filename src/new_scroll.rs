use std::{
    collections::LinkedList,
    io::Write,
    ops::{Index, Range},
    slice::SliceIndex,
};

use crossterm::QueueableCommand;
use rand::{Rng, SeedableRng};
use ratatui::{buffer::Buffer, style::Style, widgets::Widget};

use crate::command::Matcher;
use crate::pages::Pages;
use std::sync::{Arc, RwLock};

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
    pub line: Box<str>,
}

#[derive(Default, Debug)]
pub struct LinesToRenderAndView {
    pub lines: LinkedList<LineWithIdx>,
    pub view: Range<usize>,
}

#[derive(Default)]
pub struct PageAndView {
    // page: Page, // Removing this potentially unused or needing update struct field
    view: Range<usize>,
}

#[derive(Default)]
pub struct OldPageScrollState {
    show_line_numbers: bool,
    pages: Arc<RwLock<Pages>>, // Changed from page: Page to pages: Arc<RwLock<Pages>>
    page_view: Range<usize>,
    auto_scroll: bool,
    lines_being_drawn: LinesToRenderAndView,
    width: usize,
    height: usize,
    requires_redraw: bool,
    jumped_index: Option<usize>,
}

impl OldPageScrollState {
    pub fn new(pages: Arc<RwLock<Pages>>) -> Self {
        Self {
            pages,
            ..Default::default()
        }
    }

    pub fn show_line_numbers(&self) -> bool {
        self.show_line_numbers
    }

    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show;
        self.requires_redraw = true;
    }

    pub fn pages(&self) -> Arc<RwLock<Pages>> {
        self.pages.clone()
    }

    pub fn set_pages(&mut self, pages: Arc<RwLock<Pages>>) {
        self.pages = pages;
        self.requires_redraw = true;
    }

    pub fn auto_scroll(&self) -> bool {
        self.auto_scroll
    }

    pub fn set_auto_scroll(&mut self, auto: bool) {
        self.auto_scroll = auto;
    }

    pub fn requires_redraw(&self) -> bool {
        self.requires_redraw
    }

    pub fn set_requires_redraw(&mut self, redraw: bool) {
        self.requires_redraw = redraw;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// returns if the instruction requires redraw    
    pub fn apply_queue(&mut self, queue: InstructionQueue) {
        let (width, height) = (self.width, self.height);
        let pages = self.pages.read().unwrap(); // Lock pages for reading
        let pages_len = pages.lines_count();

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
                    if self.page_view.end != pages_len {
                        self.page_view.end = pages_len;

                        let mut last_lines = LinkedList::new();

                        let range = if previous_end != 0 {
                            (previous_end + 1..self.page_view.end)
                        } else {
                            (previous_end..self.page_view.end)
                        };

                        for i in range.into_iter().rev() {
                            // Using pages.get_line(i) instead of self.page[i]
                            if let Some(line_content) = pages.get_line(i) {
                                let lines = get_wrapped_lines(line_content, width);

                                // let lines = textwrap::wrap(&self.page[i], width);
                                for (l, _) in lines.into_iter().rev() {
                                    last_lines.push_front(LineWithIdx { idx: i, line: l });
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
                        }

                        {
                            let back = previous_lines.back();
                            let front = last_lines.front();
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
                if pages_len > self.page_view.end {
                    self.page_view.end += 1;
                    self.requires_redraw = true;
                }
                let mut visible_lines = LinkedList::new();

                let range = (previous_end + 1..(self.page_view.end + 1).min(pages_len));

                for i in range.into_iter().rev() {
                    if let Some(line_content) = pages.get_line(i) {
                        let lines = get_wrapped_lines(line_content, width);

                        // let lines = textwrap::wrap(&self.page[i], width);
                        for (l, _) in lines.iter().rev() {
                            visible_lines.push_front(LineWithIdx {
                                idx: i,
                                line: l.clone(),
                            });
                        }
                    }
                }

                let mut visible_end = self.lines_being_drawn.view.end + 1;
                previous_lines.append(&mut visible_lines);

                let mut visible_start = visible_end.saturating_sub(height);
                let start_idx = previous_lines
                    .iter()
                    .nth(visible_start)
                    .and_then(|x| Some(x.idx))
                    .unwrap_or(0);

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

                let mut visible_end = self.lines_being_drawn.view.end - 1;
                log::info!("pushing front lines until filling height");
                for i in (0..previous_start).into_iter().rev() {
                    self.page_view.start = i;

                    if let Some(line_content) = pages.get_line(i) {
                        let lines = get_wrapped_lines(line_content, width);
                        // let lines = textwrap::wrap(&self.page[i], width);
                        for (l, _) in lines.into_iter().rev() {
                            visible_end += 1;
                            previous_lines.push_front(LineWithIdx { idx: i, line: l });
                        }
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
                        // let lines = textwrap::wrap(&self.page[i], width);
                        if let Some(line_content) = pages.get_line(i) {
                            let lines = get_wrapped_lines(line_content, width);

                            for (l, _) in lines.into_iter().rev() {
                                visible_lines.push_front(LineWithIdx { idx: i, line: l });
                            }
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
                if pages_len <= idx {
                    log::warn!("jump to is out of range {} 0..{}", idx, pages_len);
                    return;
                }
                self.requires_redraw = true;
                self.jumped_index = Some(idx);

                if !self.lines_being_drawn.lines.is_empty() {
                    let front = self.lines_being_drawn.lines.front().unwrap();
                    let back = self.lines_being_drawn.lines.back().unwrap();
                    if front.idx <= idx && back.idx >= idx {}
                }
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

pub struct OldPageScrollWidget<'a> {
    state: &'a mut OldPageScrollState,
}

impl<'a> OldPageScrollWidget<'a> {
    pub fn new(state: &'a mut OldPageScrollState) -> Self {
        Self { state }
    }
}
impl<'a> Widget for OldPageScrollWidget<'a> {
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
            let mut string_buf = String::with_capacity(padding);
            for (y, line) in self.state.view().enumerate() {
                use std::fmt::Write;

                string_buf.clear();
                write!(string_buf, "[{}]", line.idx);

                let number_padding = (padding - 1).saturating_sub(string_buf.len());

                let style = if Some(line.idx) == self.state.jumped_index {
                    Style::new().fg(ratatui::style::Color::Yellow)
                } else {
                    Style::new()
                };

                buf.set_string(number_padding as u16, area.y + y as u16, &string_buf, style);
                buf.set_string(padding as u16, area.y + y as u16, &line.line, style);
            }
        } else {
            for (y, line) in self.state.view().enumerate() {
                let style = if Some(line.idx) == self.state.jumped_index {
                    Style::new().fg(ratatui::style::Color::Yellow)
                } else {
                    Style::new()
                };
                buf.set_string(0, area.y + y as u16, &line.line, style);
            }
        }
    }
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
                        self.render_line_partial(
                            buf,
                            area.x + padding as u16,
                            y,
                            line,
                            source_range,
                            range,
                        );
                        continue;
                    }
                }

                if let Some(range) = filter_highlight {
                    self.render_line_partial(
                        buf,
                        area.x + padding as u16,
                        y,
                        line,
                        source_range,
                        range,
                    );
                } else {
                    buf.set_string(area.x + padding as u16, y, line, style);
                }
            } else {
                if let Some(range) = state.cursor_range.as_ref() {
                    if Some(*idx) == state.cursor_idx {
                        self.render_line_partial(buf, area.x, y, line, source_range, range);
                        continue;
                    }
                }

                if let Some(range) = filter_highlight {
                    self.render_line_partial(buf, area.x, y, line, source_range, range);
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
            let highlight_style = Style::default()
                .bg(ratatui::style::Color::Yellow)
                .fg(ratatui::style::Color::Black);
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

#[test]
fn test_new_scroll() {
    use crate::pages::Pages;
    use std::fmt::Write; // Ensure Pages is imported
                         // We need Arc and RwLock for the test too
    use std::sync::{Arc, RwLock};

    // Initialize Pages
    let pages = Arc::new(RwLock::new(Pages::new(100, 5)));

    // Pass pages to OldPageScrollState::new
    let mut state = OldPageScrollState::new(pages.clone());
    state.set_auto_scroll(true);
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

        // Add line to the shared pages instance
        pages.write().unwrap().add_line(&buf);
    }

    // We can't iterate pages directly like before since it's wrapped now.
    // Also PageLineIterator worked on Page, not Pages. Pages has get_lines_iter()
    {
        let pages_read = pages.read().unwrap();
        for (idx, line) in pages_read.get_lines_iter().enumerate() {
            print!("{}: '{}'\n", idx, line);
        }
    }

    println!("^^^^^ current lines ^^^^^");

    state.apply_queue(InstructionQueue::Resize {
        width: 20,
        height: 10,
    });

    state.apply_queue(InstructionQueue::None);
    if state.requires_redraw() {
        display(&state);
    }

    state.set_auto_scroll(false);

    println!("Going down");
    loop {
        state.apply_queue(InstructionQueue::Down);
        if state.requires_redraw() {
            display(&state);
        } else {
            println!("Required no redraw");
            break;
        }
    }

    println!("Going Up");
    for i in 0..5 {
        state.apply_queue(InstructionQueue::Up);
        if state.requires_redraw() {
            display(&state);
        }
    }

    state.set_auto_scroll(true);
    println!("Autoscroll");

    state.apply_queue(InstructionQueue::None);
    if state.requires_redraw() {
        display(&state);
    }

    panic!("cuz rust don't show stdout");
}

fn display(state: &OldPageScrollState) {
    print!(
        "--------terminal (autoscroll:{}, size:{}x{})-------- real length {}, view {:?}, page_view: {:?}\n",
        state.auto_scroll(),
        state.width(),
        state.height(),
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

#[test]
fn test_new_page_scroll_widget() {
    use crate::pages::Pages;
    use std::fmt::Write;
    use std::sync::{Arc, RwLock};

    let pages = Arc::new(RwLock::new(Pages::new(100, 5)));
    let mut state = PageScrollState::new(pages.clone());
    state.set_size(20, 10);
    state.toggle_line_numbers(); // show line numbers

    for i in 0..15 {
        let mut buf = String::new();
        for _ in 0..(i % 5 + 1) {
            write!(&mut buf, "L{} ", i).unwrap();
        }
        pages.write().unwrap().add_line(&buf);
    }

    println!("Testing NewPageScrollWidget with autoscroll=true");
    display_new(&state);

    state.toggle_autoscroll();
    println!(
        "Testing NewPageScrollWidget with autoscroll=false (currently just shows last lines too)"
    );
    display_new(&state);

    state.toggle_line_numbers();
    println!("Testing NewPageScrollWidget without line numbers");
    display_new(&state);

    println!("Testing scroll_up (disables autoscroll)");
    state.scroll_up();
    display_new(&state);

    println!("Testing jump_to(5)");
    state.jump_to(5);
    display_new(&state);

    println!("Testing scroll_down (re-enables autoscroll if at bottom)");
    for _ in 0..15 {
        state.scroll_down();
    }
    display_new(&state);

    // panic!("To show output");
}

fn display_new(state: &PageScrollState) {
    let area = ratatui::prelude::Rect::new(0, 0, state.width as u16, state.height as u16);
    let mut buffer = Buffer::empty(area);
    PageScrollWidget(state).render(area, &mut buffer);

    println!(
        "-------- PageScrollWidget (size:{}x{}) --------",
        state.width, state.height
    );
    for y in 0..area.height {
        let mut line = String::new();
        for x in 0..area.width {
            let cell = &buffer[(x, y)];
            line.push(cell.symbol().chars().next().unwrap_or(' '));
        }
        println!("{}", line);
    }
    println!("---------------------------------------------------");
}

use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::Style,
    widgets::{Block, Paragraph, Widget},
    Frame,
};

use crate::TuiMode;

pub struct ScrollView<'a> {
    state: &'a mut State,
}

impl<'a> ScrollView<'a> {
    pub fn new(state: &'a mut State) -> Self {
        Self { state }
    }
}

impl<'a> Widget for ScrollView<'a> {
    fn render(self, area: ratatui::prelude::Rect, buffer: &mut Buffer)
    where
        Self: Sized,
    {
        let height = area.height;
        let width = area.width;

        let lines = textwrap::wrap(self.state.get_content(), width as usize);
        let mut vertical_position = self.state.vertical_position;

        if self.state.auto_scroll {
            vertical_position = lines.len().checked_sub(height as usize).unwrap_or(0);
        }
        let start = (vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());

        let visible_lines = &lines[start..end];

        for (y, line) in visible_lines.iter().enumerate() {
            let f_line = format!("{:>5} {}", start + y, line,);

            buffer.set_string(0, area.y + y as u16, &f_line, Style::new());
        }

        if end == lines.len() {
            self.state.set_auto_scroll(true);
        }
        self.state.vertical_position = vertical_position;
    }
}

pub struct State {
    vertical_position: usize,
    content: String,
    pub mode: TuiMode,
    pub auto_scroll: bool,
    pub command: String,
}

impl State {
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

    pub fn get_content(&self) -> &str {
        &self.content
    }

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

pub fn main_pane_draw(frame: &mut Frame, state: &mut State) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let title = Paragraph::new(state.command.as_str()).block(Block::bordered().title("Filter"));
    frame.render_widget(title, title_area);

    frame.render_widget(ScrollView::new(state), main_area);
    let status = format!(
        "MODE: {:?}, AUTOSCROLL: {} '/': search ^c: clear search ^d: exit",
        state.mode, state.auto_scroll
    );
    frame.render_widget(Block::bordered().title(status.as_str()), status_area);
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

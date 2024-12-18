use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::Style,
    widgets::{Block, Widget},
    Frame,
};

pub struct ScrollView {
    content: String,
    vertical_position: usize,
}

impl ScrollView {
    pub fn new(content: String, vertical_position: usize) -> Self {
        Self {
            content,
            vertical_position,
        }
    }
}

impl Widget for ScrollView {
    fn render(self, area: ratatui::prelude::Rect, buffer: &mut Buffer)
    where
        Self: Sized,
    {
        let height = area.height;
        let width = area.width;

        let lines = textwrap::wrap(self.content.as_str(), width as usize);

        let start = (self.vertical_position).min(lines.len());
        let end = (start + height as usize).min(lines.len());

        let visible_lines = &lines[start..end];

        for (y, line) in visible_lines.iter().enumerate() {
            buffer.set_string(0, y as u16, line, Style::new());
        }
    }
}

pub fn main_pane_draw(frame: &mut Frame, lines: &[String], vertical_position: usize) {
    // let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    // let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let main_area = frame.area();

    frame.render_widget(
        ScrollView::new(lines.join("\n"), vertical_position),
        main_area,
    );

    // let status = format!("No. of lines {} ", lines.len());
    // frame.render_widget(Block::bordered().title("Filter"), title_area);
    // frame.render_widget(Block::bordered().title(status.as_str()), status_area);
}

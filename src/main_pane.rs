use ratatui::{
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    widgets::{Block, Paragraph},
    Frame,
};

use crate::scroll_view::{ScrollView, ScrollViewState};

pub fn main_pane_draw(frame: &mut Frame, state: &mut ScrollViewState) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let title = Paragraph::new(state.command.as_str()).block(Block::bordered().title("Filter"));
    frame.render_widget(title, title_area);

    frame.render_widget(ScrollView::new(state), main_area);
    let status = format!(
        "MODE: {:?}, AUTOSCROLL: {} '/': search ^c: clear search ^q: exit",
        state.mode, state.auto_scroll
    );
    frame.render_widget(Block::bordered().title(status.as_str()), status_area);
}

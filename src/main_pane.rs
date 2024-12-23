use ratatui::{
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    new_scroll::{PageScrollState, PageScrollWidget},
    scroll_view::{AppState, ScrollState, ScrollView},
};

pub fn main_pane_draw(
    frame: &mut Frame,
    title: &str,
    app_state: &mut AppState,
    scroll_state: &mut ScrollState,
) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let title = Paragraph::new(app_state.command.as_str()).block(Block::bordered().title(title));
    frame.render_widget(title, title_area);
    frame.render_widget(ScrollView::new(app_state, scroll_state), main_area);
    let status = format!(
        "mode: {:?}, scroll: {} | n: toggle numbers | '/': search | ^c: clear | ^q: exit",
        app_state.mode, scroll_state.auto_scroll
    );
    frame.render_widget(Block::bordered().title(status.as_str()), status_area);
}
pub fn main_pane_with_page_scroll_draw(
    frame: &mut Frame,
    title: &str,
    page_scroll_state: &mut PageScrollState,
) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());
    let title = Paragraph::new("search:").block(Block::bordered().title(title));
    frame.render_widget(title, title_area);

    // if page_scroll_state.requires_redraw {
    frame.render_widget(PageScrollWidget::new(page_scroll_state), main_area);
    // page_scroll_state.requires_redraw = false;
    // }

    let status = format!(
        "mode: NORMAL, scroll: {} | n: toggle numbers | '/': search | ^c: clear | ^q: exit",
        page_scroll_state.auto_scroll
    );
    frame.render_widget(Block::bordered().title(status.as_str()), status_area);
}

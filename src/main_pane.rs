use ratatui::{
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::Style,
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::{command::FilterTitleWidget, new_scroll::PageScrollWidget};

pub fn main_pane_with_page_scroll_draw(frame: &mut Frame, app: &mut crate::app::App) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());

    let active_filter = app.scroll_state.filter().map(|f| f.to_string());
    let active_search = app.search_query.as_ref().map(|s| s.to_string());
    frame.render_widget(
        FilterTitleWidget::new(&app.cmd_builder, active_filter, active_search, &app.title),
        title_area,
    );
    app.scroll_state
        .set_size(main_area.width as usize, main_area.height as usize);
    frame.render_widget(PageScrollWidget(&app.scroll_state), main_area);

    if app.error_timer.error.is_empty() {
        let scroll_status = if app.scroll_state.auto_scroll() {
            "Autoscroll: ON"
        } else {
            "Autoscroll: OFF"
        };
        let line_numbers_status = if app.scroll_state.show_line_numbers() {
            "Numbers: ON"
        } else {
            "Numbers: OFF"
        };
        let status = format!("{} | {} | <space> menu", scroll_status, line_numbers_status);
        frame.render_widget(Block::bordered().title(status), status_area);
    } else {
        log::warn!("drawing error {}", app.error_timer.error);
        frame.render_widget(
            Block::bordered()
                .title(app.error_timer.error.as_str())
                .style(Style::new().fg(ratatui::style::Color::Red)),
            status_area,
        );
    }
}

pub fn draw_space_menu(frame: &mut Frame) {
    const MENU_CONTENT: &'static str =
        "s search\nr regex\ni ignore\nf filter\nn numbers\na autoscroll\nc clear\n: jump to\n";
    let horizontal = Layout::horizontal([Min(0), Length(20)]).margin(8);
    let [_, menu_area] = horizontal.areas(frame.area());
    frame.render_widget(Clear, menu_area);
    let paragraph = Paragraph::new(MENU_CONTENT).block(Block::bordered().title("Menu"));
    frame.render_widget(paragraph, menu_area);
}

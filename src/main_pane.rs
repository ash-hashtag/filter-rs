use ratatui::{
    layout::{
        Constraint::{Length, Min},
        Layout,
    },
    style::Style,
    widgets::{Block, Clear, Paragraph},
    Frame,
};

use crate::{
    command::{CommandBuilder, FilterTitleWidget},
    new_scroll::{PageScrollState, PageScrollWidget},
    scroll_view::{AppState, ScrollState, ScrollView},
};

pub fn main_pane_with_page_scroll_draw(frame: &mut Frame, app: &mut crate::app::App) {
    let vertical = Layout::vertical([Length(3), Min(0), Length(1)]);
    let [title_area, main_area, status_area] = vertical.areas(frame.area());

    frame.render_widget(
        FilterTitleWidget::new(&app.cmd_builder, &app.title),
        title_area,
    );
    app.scroll_state
        .set_size(main_area.width as usize, main_area.height as usize);
    frame.render_widget(PageScrollWidget(&app.scroll_state), main_area);

    if app.error_timer.error.is_empty() {
        let status = "<space> to toggle menu";
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
        "s search\nr regex\ni ignore\nf filter\nn numbers\nc clear\n: jump to\n";
    let horizontal = Layout::horizontal([Min(0), Length(20)]).margin(8);
    let [_, menu_area] = horizontal.areas(frame.area());
    frame.render_widget(Clear, menu_area);
    let paragraph = Paragraph::new(MENU_CONTENT).block(Block::bordered().title("Menu"));
    frame.render_widget(paragraph, menu_area);
}

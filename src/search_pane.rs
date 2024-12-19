use ratatui::Frame;

use crate::scroll_view::AppState;

// pub struct SearchPane {
//     search_term: String,
//     initial_lines: Vec<String>,
//     stream_of_lines: UnboundedReceiver<String>,
// }

// impl SearchPane {
//     pub fn new(
//         search_term: impl Into<String>,
//         initial_lines: Vec<String>,
//         stream_of_lines: UnboundedReceiver<String>,
//     ) -> Self {
//         Self {
//             search_term: search_term.into(),
//             initial_lines,
//             stream_of_lines,
//         }
//     }

//     pub async fn run(&mut self) -> anyhow::Result<()> {
//         let mut writer = std::io::stdout();

//         while let Some(line) = self.stream_of_lines.recv().await {
//             if let Some(index) = line.find(&self.search_term) {
//                 let styled = self
//                     .search_term
//                     .clone()
//                     .on(style::Color::Yellow)
//                     .attribute(style::Attribute::Bold);
//                 writer.write(&line[0..index].as_bytes())?;
//                 writer.queue(style::Print(styled))?;
//                 writer.write(&line[index + self.search_term.len()..].as_bytes())?;
//             }
//         }
//         Ok(())
//     }

//     pub fn close(&mut self) {
//         self.search_term.clear();
//         self.initial_lines.clear();
//         if !self.stream_of_lines.is_closed() {
//             self.stream_of_lines.close();
//         }
//     }
// }

// impl Drop for SearchPane {
//     fn drop(&mut self) {
//         self.close();
//     }
// }

pub fn search_pane_draw(frame: &mut Frame, state: &mut AppState) {}

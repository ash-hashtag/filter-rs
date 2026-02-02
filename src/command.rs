use std::{collections::LinkedList, ops::Range};

use ratatui::widgets::{Block, Paragraph, Widget};
use regex::Regex;

use crate::pages::Page;

pub enum SearchPattern {
    Regex(String),
    Substring(String),
}

impl SearchPattern {
    pub fn is_match(&self, s: &str) -> Option<Range<usize>> {
        match self {
            SearchPattern::Regex(r) => {
                let regexp = Regex::new(r).ok()?;
                let mat = regexp.find(s)?;

                return Some(mat.start()..mat.end());
            }
            SearchPattern::Substring(substr) => {
                let start = s.find(substr)?;
                return Some(start..start + substr.len());
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum CommandType {
    #[default]
    None,
    Ignore,
    Search,
    Regex,
    JumpTo,
}

#[derive(Default, Debug)]
pub struct CommandBuilder {
    pub cmd_type: CommandType,
    pub cmd: String,
}

impl CommandBuilder {
    pub fn clear(&mut self) {
        self.cmd.clear();
        self.cmd_type = CommandType::None;
    }
}

pub struct FilterTitleWidget<'a> {
    cmd: &'a CommandBuilder,
    title: &'a str,
}

impl<'a> FilterTitleWidget<'a> {
    pub fn new(cmd: &'a CommandBuilder, title: &'a str) -> Self {
        Self { cmd, title }
    }
}

impl<'a> Widget for FilterTitleWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut command = String::with_capacity(self.cmd.cmd.len() + 16);
        let prefix = match self.cmd.cmd_type {
            CommandType::None => "None",
            CommandType::Ignore => "Ignore",
            CommandType::Search => "Search",
            CommandType::Regex => "Regex",
            CommandType::JumpTo => "JumpTo",
        };
        command.push_str(prefix);
        command.push(':');
        command.push_str(&self.cmd.cmd);

        let title = Paragraph::new(command).block(Block::bordered().title(self.title));

        title.render(area, buf);
    }
}

impl CommandBuilder {
    pub fn build(self) -> Command {
        match self.cmd_type {
            CommandType::Ignore => Command::Ignore(SearchPattern::Substring(self.cmd)),
            CommandType::Search => Command::SearchFor(SearchPattern::Substring(self.cmd)),
            CommandType::Regex => Command::SearchFor(SearchPattern::Regex(self.cmd)),
            _ => unreachable!(),
        }
    }
}

pub enum Command {
    Ignore(SearchPattern),
    SearchFor(SearchPattern),
    FuzzySearch(String),

    Any(Vec<Command>),
    Every(Vec<Command>),
}

impl Command {
    pub fn is_match(&self, s: &str) -> Option<Range<usize>> {
        match self {
            Command::Ignore(search_pattern) => search_pattern.is_match(s),
            Command::SearchFor(search_pattern) => search_pattern.is_match(s),
            Command::FuzzySearch(_) => todo!(),
            Command::Any(vec) => {
                for cmd in vec {
                    if let Some(mat) = cmd.is_match(s) {
                        return Some(mat);
                    }
                }
                return None;
            }
            Command::Every(vec) => {
                let mut mat = None;
                for cmd in vec {
                    if let Some(new_mat) = cmd.is_match(s) {
                        mat = Some(new_mat);
                    } else {
                        return None;
                    }
                }

                return mat;
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct LineWithIdx {
    pub idx: usize,
    pub line: String,
    pub highlight: Range<usize>,
}

#[derive(Default, Debug)]
pub struct LinesToRenderAndView {
    pub lines: LinkedList<LineWithIdx>,
    pub view: Range<usize>,
}

pub struct CommandsScrollState {
    page: Page,
    page_view: Range<usize>,
    pub auto_scroll: bool,
    lines_being_drawn: LinesToRenderAndView,
    width: usize,
    height: usize,
    pub requires_redraw: bool,
    command: Command,
}

impl CommandsScrollState {
    pub fn add_line(&mut self, line: &str) {
        let idx = self.page.add_line(line);

        if let Some(mat) = self.command.is_match(line) {
            let line = LineWithIdx {
                idx,
                line: line.to_string(),
                highlight: mat,
            };
        }
    }
}

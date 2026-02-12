use std::ops::Range;

use ratatui::widgets::{Block, Paragraph, Widget};
use regex::Regex;

pub trait Matcher: Sync {
    fn is_match(&self, s: &str) -> Option<Range<usize>>;
}

#[derive(Debug, Clone)]
pub enum SearchPattern {
    Regex(Regex),
    Substring(String),
}

impl Matcher for SearchPattern {
    fn is_match(&self, s: &str) -> Option<Range<usize>> {
        match self {
            SearchPattern::Regex(regexp) => {
                let mat = regexp.find(s)?;
                Some(mat.start()..mat.end())
            }
            SearchPattern::Substring(substr) => {
                let start = s.find(substr)?;
                Some(start..start + substr.len())
            }
        }
    }
}

impl std::fmt::Display for SearchPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchPattern::Regex(regexp) => write!(f, "{}", regexp),
            SearchPattern::Substring(substr) => write!(f, "{}", substr),
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
    Filter,
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
    active_filter: Option<String>,
    active_search: Option<String>,
    title: &'a str,
}

impl<'a> FilterTitleWidget<'a> {
    pub fn new(
        cmd: &'a CommandBuilder,
        active_filter: Option<String>,
        active_search: Option<String>,
        title: &'a str,
    ) -> Self {
        Self {
            cmd,
            active_filter,
            active_search,
            title,
        }
    }
}

impl<'a> Widget for FilterTitleWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut command = String::with_capacity(self.cmd.cmd.len() + 16);
        let prefix = match self.cmd.cmd_type {
            CommandType::None => {
                if let Some(f) = &self.active_filter {
                    command.push_str("Filter: ");
                    command.push_str(f);
                } else if let Some(s) = &self.active_search {
                    command.push_str("Search: ");
                    command.push_str(s);
                } else {
                    command.push_str("None");
                }
                ""
            }
            CommandType::Ignore => "Ignore",
            CommandType::Search => "Search",
            CommandType::Regex => "Regex",
            CommandType::JumpTo => "JumpTo",
            CommandType::Filter => "Filter",
        };
        if !prefix.is_empty() {
            command.push_str(prefix);
            command.push_str(": ");
            command.push_str(&self.cmd.cmd);
        }

        let title = Paragraph::new(command).block(Block::bordered().title(self.title));

        title.render(area, buf);
    }
}

impl CommandBuilder {
    pub fn build(&self) -> Option<Command> {
        match self.cmd_type {
            CommandType::Ignore => {
                Some(Command::Ignore(SearchPattern::Substring(self.cmd.clone())))
            }
            CommandType::Search => Some(Command::SearchFor(SearchPattern::Substring(
                self.cmd.clone(),
            ))),
            CommandType::Regex => {
                let regex = Regex::new(&self.cmd).ok()?;
                Some(Command::SearchFor(SearchPattern::Regex(regex)))
            }
            CommandType::Filter => Some(Command::SearchFor(SearchPattern::Substring(
                self.cmd.clone(),
            ))),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Ignore(SearchPattern),
    SearchFor(SearchPattern),
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Ignore(p) => write!(f, "Ignore:{}", p),
            Command::SearchFor(p) => write!(f, "{}", p),
        }
    }
}

impl Matcher for Command {
    fn is_match(&self, s: &str) -> Option<Range<usize>> {
        match self {
            Command::Ignore(search_pattern) => search_pattern.is_match(s),
            Command::SearchFor(search_pattern) => search_pattern.is_match(s),
        }
    }
}

use std::{collections::LinkedList, ops::Range};

use regex::Regex;

use crate::pages::Page;

pub enum SearchPattern {
    Regex(String),
    Substring(String),
}

impl SearchPattern {
    pub fn get_mut_str(&mut self) -> &mut String {
        match self {
            SearchPattern::Regex(s) => s,
            SearchPattern::Substring(s) => s,
        }
    }

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

pub enum Command {
    Ignore(SearchPattern),
    SearchFor(SearchPattern),
    FuzzySearch(String),

    Any(Vec<Command>),
    Every(Vec<Command>),
}

impl Command {
    pub fn get_mut_str(&mut self) -> &mut String {
        match self {
            Command::Ignore(s) => s.get_mut_str(),
            Command::SearchFor(s) => s.get_mut_str(),
            Command::FuzzySearch(s) => s,
            Command::Any(vec) => todo!(),
            Command::Every(vec) => todo!(),
        }
    }

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

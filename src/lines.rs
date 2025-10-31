use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::double_linked_list::Node;

pub struct LineAndNumber {
    line_no: usize,
    line: String,
}
impl LineAndNumber {
    pub fn new(line_no: usize, line: String) -> Self {
        Self { line_no, line }
    }
}

pub struct Lines {
    head: Node<LineAndNumber>,
    start: Node<LineAndNumber>,
    max_number_of_lines: usize,
}

impl Lines {
    pub fn new(max_number_of_lines: usize) -> Self {
        let start = Node::new(LineAndNumber::new(0, String::new()));
        Self {
            head: start.clone(),
            start,
            max_number_of_lines,
        }
    }

    pub fn push(&mut self, line: String) {
        let line_no = self.head.value().line_no;
        let next = Node::new(LineAndNumber::new(line_no + 1, line));
        self.head.push_next(next.clone());
        self.head = next;

        if self.start.value().line_no + self.max_number_of_lines >= line_no {
            if let Some(mut next) = self.start.next() {
                next.pop_front();
                self.start = next;
            }
        }
    }

    pub fn search_for(&self, word: String) {}
}

pub struct SearchIterator {
    search: String,
    cursor: Node<LineAndNumber>,
}

impl Iterator for SearchIterator {
    type Item = Rc<LineAndNumber>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.cursor.value();
            if value.line.contains(&self.search) {
                self.cursor = self.cursor.next()?;
                return Some(value);
            }
        }

        None
    }
}

impl DoubleEndedIterator for SearchIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let value = self.cursor.value();
            if value.line.contains(&self.search) {
                self.cursor = self.cursor.prev()?;
                return Some(value);
            }
        }
        None
    }
}

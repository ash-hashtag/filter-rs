use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct Page {
    line_start_number: usize,
    lines: Vec<String>,
}
impl Page {
    pub fn new(line_start_number: usize, lines: Vec<String>) -> Self {
        Self {
            line_start_number,
            lines,
        }
    }
}

#[derive(Debug)]
pub struct Pages {
    pages: RefCell<Vec<Page>>,

    max_lines_per_page: usize,
    max_number_of_pages: usize,
}

impl Pages {
    pub fn new(max_lines_per_page: usize, max_number_of_pages: usize) -> Self {
        Self {
            pages: RefCell::new(Vec::new()),

            max_lines_per_page,
            max_number_of_pages,
        }
    }

    pub fn add_line(&self, line: String) {
        let mut pages = self.pages.borrow_mut();

        if pages.is_empty() {
            pages.push(Page::new(0, vec![line]));
            return;
        }

        let last_page = pages.last_mut().unwrap();
        let last_line_number = last_page.lines.len() + last_page.line_start_number;

        if last_page.lines.len() < self.max_lines_per_page {
            last_page.lines.push(line);
        } else {
            pages.push(Page::new(last_line_number, vec![line]));
        }
    }

    pub fn get_line(&self, line_number: usize) -> Option<String> {
        let pages = self.pages.borrow();
        let idx = match pages.binary_search_by_key(&line_number, |x| x.line_start_number) {
            Ok(v) => v,
            Err(v) => v.checked_sub(1)?,
        };
        let page = pages.get(idx)?;
        let line = page.lines.get(line_number - page.line_start_number)?;

        Some(line.to_owned())
    }
}

#[test]
fn test_pages() {
    let pages = Pages::new(10, 4);
    for i in 0..50 {
        pages.add_line(i.to_string());
    }

    for i in 0..50 {
        let line = pages.get_line(i).unwrap();

        assert!(i.to_string() == line);
    }

    let mut iter = SearchIterator::new(pages, "4", 0);

    let (line_no, line) = iter.next().unwrap();
    eprintln!("{} {} {}", line_no, iter.current_line_number, line);
}

#[test]
fn test_search_iterator_bidirectional() {
    let pages = Pages::new(5, 10);
    for i in 0..20 {
        pages.add_line(format!("line{}", i));
    }

    let mut iter = SearchIterator::new(pages, "line1", 10);
    
    // Forward iteration
    let (line_no, line) = iter.next().unwrap();
    assert_eq!(line_no, 10);
    assert_eq!(line, "line10");
    
    let (line_no, line) = iter.next().unwrap();
    assert_eq!(line_no, 11);
    assert_eq!(line, "line11");
    
    // Backward iteration from end
    let (line_no, line) = iter.next_back().unwrap();
    assert_eq!(line_no, 19);
    assert_eq!(line, "line19");
    
    let (line_no, line) = iter.next_back().unwrap();
    assert_eq!(line_no, 18);
    assert_eq!(line, "line18");
}

pub struct SearchIterator {
    pages: Rc<Pages>,
    search_for: String,

    current_line_number: usize,
}

impl SearchIterator {
    pub fn new(
        pages: impl Into<Rc<Pages>>,
        search_for: impl Into<String>,
        current_line_number: usize,
    ) -> Self {
        Self {
            pages: pages.into(),
            search_for: search_for.into(),
            current_line_number,
        }
    }
}

impl Iterator for SearchIterator {
    type Item = (usize, String);

    fn next(&mut self) -> Option<Self::Item> {
        let pages = self.pages.pages.borrow();

        let idx =
            match pages.binary_search_by_key(&self.current_line_number, |x| x.line_start_number) {
                Ok(v) => v,
                Err(v) => v.checked_sub(1)?,
            };

        let mut page = pages.get(idx)?;

        loop {
            let idx_inside_page = self.current_line_number - page.line_start_number;
            let line = page.lines.get(idx_inside_page)?;

            self.current_line_number += 1;
            if line.contains(&self.search_for) {
                return Some((self.current_line_number - 1, line.to_owned()));
            }

            if idx_inside_page + 1 == page.lines.len() {
                if idx + 1 == pages.len() {
                    return None;
                }
                page = pages.get(idx + 1)?;
            }
        }

        return None;
    }
}

impl DoubleEndedIterator for SearchIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let pages = self.pages.pages.borrow();
        
        // Start from the last line if we haven't started backwards iteration
        let mut search_line = if self.current_line_number == 0 {
            // Find the total number of lines
            let last_page = pages.last()?;
            last_page.line_start_number + last_page.lines.len() - 1
        } else {
            self.current_line_number.checked_sub(1)?
        };

        loop {
            let idx = match pages.binary_search_by_key(&search_line, |x| x.line_start_number) {
                Ok(v) => v,
                Err(v) => v.checked_sub(1)?,
            };

            let page = pages.get(idx)?;
            let idx_inside_page = search_line - page.line_start_number;
            let line = page.lines.get(idx_inside_page)?;

            if line.contains(&self.search_for) {
                return Some((search_line, line.to_owned()));
            }

            if search_line == 0 {
                return None;
            }
            search_line -= 1;
        }
    }
}

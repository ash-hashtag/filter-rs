use std::{fmt::Write, ops::Deref};

pub struct RecycleList<T> {
    inner: Vec<T>,
    max_capacity: usize,
    shrink_to: usize,
}

impl<T> RecycleList<T> {
    pub fn new(shrink_to: usize, max_capacity: usize) -> Self {
        assert!(shrink_to < max_capacity);
        Self {
            inner: Vec::with_capacity(max_capacity),
            max_capacity,
            shrink_to,
        }
    }

    pub fn push(&mut self, element: T) {
        if self.inner.len() >= self.max_capacity {
            self.purge();
        }
        self.inner.push(element);
    }

    pub fn purge(&mut self) {
        for i in 0..self.shrink_to {
            let s = self.inner.pop().unwrap();
            let index = self.shrink_to - i - 1;
            self.inner[index] = s;
        }
        self.inner.truncate(self.shrink_to);
    }
}

impl<T> Deref for RecycleList<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct Pages {
    lines: Vec<String>,
    max_capacity: usize,
    purge_count: usize,
}

impl Default for Pages {
    fn default() -> Self {
        Self::new(4_000, 1_500)
    }
}

impl Pages {
    pub fn new(max_capacity: usize, purge_count: usize) -> Self {
        assert!(max_capacity > purge_count * 2);
        let lines = Vec::with_capacity(max_capacity);

        Self {
            lines,
            max_capacity,
            purge_count,
        }
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        if self.lines.len() == self.max_capacity {
            self.purge();
        }
    }

    pub fn purge(&mut self) {
        for i in 0..self.purge_count {
            let s = self.lines.pop().unwrap();
            let index = self.purge_count - i - 1;
            self.lines[index] = s;
        }

        self.lines.truncate(self.purge_count);
    }

    pub fn get_lines<'a>(&'a self) -> PagesLineIterator<'a> {
        PagesLineIterator {
            pages: self,
            inner_cursor: 0,
        }
    }

    pub fn get_pages(&self) -> &[String] {
        &self.lines
    }
}

pub struct PagesLineIterator<'a> {
    pages: &'a Pages,
    inner_cursor: usize,
}

impl<'a> Iterator for PagesLineIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner_cursor >= self.pages.lines.len() {
            return None;
        }
        self.inner_cursor += 1;
        Some(&self.pages.lines[self.inner_cursor - 1])
    }
}

impl<'a> DoubleEndedIterator for PagesLineIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.inner_cursor >= self.pages.lines.len() {
            return None;
        }
        self.inner_cursor += 1;
        return Some(&self.pages.lines[self.pages.lines.len() - self.inner_cursor]);
    }
}

#[derive(Default)]
pub struct Page {
    inner: String,
    indices: Vec<Segment>,
}

#[derive(Copy, Clone)]
pub struct Segment {
    start_position: u32,
    terminal_width: u32,
}

#[derive(Debug)]
pub struct StrSegment<'a> {
    terminal_width: u32,
    s: &'a str,
}

impl Page {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: String::with_capacity(cap),
            indices: Vec::new(),
        }
    }

    pub fn add_str(&mut self, s: &str) {
        for line in s.lines() {
            let start_position = self.inner.len() as u32;
            let terminal_width =
                reformat_line_into_and_get_terminal_width(line, &mut self.inner) as u32;
            self.indices.push(Segment {
                start_position,
                terminal_width,
            });
        }
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn get(&self, idx: usize) -> Option<StrSegment> {
        let segment = self.indices.get(idx)?;
        let start = segment.start_position as usize;
        let end = self
            .indices
            .get(idx + 1)
            .and_then(|x| Some(x.start_position as usize))
            .unwrap_or(self.inner.len());

        Some(StrSegment {
            terminal_width: segment.terminal_width,
            s: &self.inner[start..end],
        })
    }
}

#[allow(unused)]
fn test_pages() {
    let mut pages = Pages::new(100, 30);
    let mut buf = String::new();

    for i in 0..1000 {
        let s = i.to_string();
        for _ in 0..1 {
            buf += s.as_str();
        }
        pages.add_line(&buf);
        buf.clear();
    }

    for line in pages.get_lines() {
        println!("{}", line);
    }

    println!("---------------");

    for line in pages.get_lines().rev() {
        println!("{}", line);
    }

    println!("{:?}", pages.get_pages());
}

pub fn reformat_line_into_and_get_terminal_width(s: &str, w: &mut impl Write) -> usize {
    let mut width = 0;
    for c in s.chars() {
        match c {
            '\t' => {
                w.write_str("    ").unwrap();
                width += 4;
            }
            '\r' => {
                w.write_char('\r').unwrap();
            }
            _ => {
                w.write_char(c).unwrap();
                width += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            }
        }
    }

    width
}

// pub struct TerminalLine {
//     s: RcStr,
//     terminal_width: u16,
// }

// impl TerminalLine {
//     pub fn new(s: &str) -> Self {
//         let mut width = 0;
//         for c in s.chars() {
//             width += match c {
//                 '\t' => 4,
//                 '\r' => 0,
//                 _ => unicode_width::UnicodeWidthChar::width(c).unwrap(),
//             } as u16;
//         }

//         Self {
//             s: RcStr::from(s),
//             terminal_width: width,
//         }
//     }
// }

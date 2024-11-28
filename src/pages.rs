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
    pages: Vec<Page>,
    page_capacity: usize,
}

impl Default for Pages {
    fn default() -> Self {
        Self::new(40_000, 10)
    }
}

impl Pages {
    pub fn new(page_capacity: usize, page_count: usize) -> Self {
        let mut pages = Vec::with_capacity(page_count);
        pages.push(Page::with_capacity(page_capacity));
        Self {
            page_capacity,
            pages,
        }
    }

    pub fn add_line(&mut self, s: &str) {
        if self.pages.last_mut().unwrap().add_str_only_if_in_cap(s) {
            return;
        }

        if self.pages.len() == self.pages.capacity() {
            let mut page = self.pages.remove(0);
            page.clear();
            page.add_str(s);
            self.pages.push(page);
        } else {
            let mut page = Page::with_capacity(self.page_capacity);
            page.add_str(s);
            self.pages.push(page);
        }
    }

    pub fn get_lines<'a>(&'a self) -> PagesLineIterator<'a> {
        PagesLineIterator {
            pages: self,
            inner_cursor: 0,
        }
    }

    pub fn get_line(&self, idx: usize) -> Option<StrSegment> {
        let mut rdx = 0;

        for page in &self.pages {
            if rdx + page.len() > idx {
                return page.get(idx - rdx);
            }

            rdx += page.len();
        }

        return None;
    }

    pub fn len(&self) -> usize {
        let mut size = 0;

        for page in &self.pages {
            size += page.len();
        }

        size
    }

    pub fn get_lines_per_frame(&self, width: u16, height: u16) -> String {
        let mut buf = String::with_capacity((width * height) as usize);

        let mut height_filled = 0;
        let mut lines_count = 0;

        for line in self.get_lines().rev() {
            let mut height_this_line_takes = line.terminal_width / width as u32;
            if line.terminal_width % width as u32 > 0 {
                height_this_line_takes += 1;
            }

            height_filled += height_this_line_takes;
            if height_filled >= height as u32 {
                break;
            }

            lines_count += 1;
        }

        let end_idx = self.len();
        let start_idx = end_idx - lines_count;

        for i in start_idx..end_idx {
            buf += self.get_line(i).unwrap().s;
            buf += "\n";
        }

        buf
    }
}

pub struct PagesLineIterator<'a> {
    pages: &'a Pages,
    inner_cursor: usize,
}

impl<'a> Iterator for PagesLineIterator<'a> {
    type Item = StrSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let s = self.pages.get_line(self.inner_cursor);
        self.inner_cursor += 1;
        s
    }
}

impl<'a> DoubleEndedIterator for PagesLineIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.inner_cursor == self.pages.len() {
            return None;
        }

        let s = self
            .pages
            .get_line(self.pages.len() - self.inner_cursor - 1);

        self.inner_cursor += 1;

        return s;
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
    pub terminal_width: u32,
    pub s: &'a str,
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

    pub fn add_str_only_if_in_cap(&mut self, s: &str) -> bool {
        if self.inner.len() + s.len() > self.inner.capacity() {
            return false;
        }
        self.add_str(s);
        return true;
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

    pub fn clear(&mut self) {
        self.inner.clear();
        self.indices.clear();
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

    // for line in pages.get_lines() {
    //     println!("{}", line);
    // }

    // println!("---------------");

    // for line in pages.get_lines().rev() {
    //     println!("{}", line);
    // }

    // println!("{:?}", pages.get_pages());
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

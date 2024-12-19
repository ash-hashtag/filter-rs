// pub struct RecycleList<T> {
//     inner: Vec<T>,
//     max_capacity: usize,
//     shrink_to: usize,
// }

// impl<T> RecycleList<T> {
//     pub fn new(shrink_to: usize, max_capacity: usize) -> Self {
//         assert!(shrink_to < max_capacity);
//         Self {
//             inner: Vec::with_capacity(max_capacity),
//             max_capacity,
//             shrink_to,
//         }
//     }

//     pub fn push(&mut self, element: T) {
//         if self.inner.len() >= self.max_capacity {
//             self.purge();
//         }
//         self.inner.push(element);
//     }

//     pub fn purge(&mut self) {
//         for i in 0..self.shrink_to {
//             let s = self.inner.pop().unwrap();
//             let index = self.shrink_to - i - 1;
//             self.inner[index] = s;
//         }
//         self.inner.truncate(self.shrink_to);
//     }
// }

// impl<T> Deref for RecycleList<T> {
//     type Target = Vec<T>;

//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

use std::ops::{Index, Range};

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

    pub fn get_lines_iter<'a>(&'a self) -> PagesLineIterator<'a> {
        PagesLineIterator {
            pages: self,
            inner_cursor: 0,
        }
    }

    pub fn get_line(&self, idx: usize) -> Option<&str> {
        let mut rdx = 0;

        for page in &self.pages {
            if rdx + page.len() > idx {
                return page.get_at(idx - rdx);
            }

            rdx += page.len();
        }

        return None;
    }

    pub fn lines_count(&self) -> usize {
        let mut size = 0;

        for page in &self.pages {
            size += page.len();
        }

        size
    }

    // pub fn get_lines_per_frame(&self, width: u16, height: u16) -> String {
    //     let mut buf = String::with_capacity((width * height) as usize);

    //     let mut height_filled = 0;
    //     let mut lines_count = 0;

    //     for line in self.get_lines().rev() {
    //         let mut height_this_line_takes = line.terminal_width / width as u32;
    //         if line.terminal_width % width as u32 > 0 {
    //             height_this_line_takes += 1;
    //         }

    //         height_filled += height_this_line_takes;
    //         if height_filled >= height as u32 {
    //             break;
    //         }

    //         lines_count += 1;
    //     }

    //     let end_idx = self.len();
    //     let start_idx = end_idx - lines_count;

    //     for i in start_idx..end_idx {
    //         buf += self.get_line(i).unwrap().s;
    //         buf += "\n";
    //     }

    //     buf
    // }
}

pub struct PagesLineIterator<'a> {
    pages: &'a Pages,
    inner_cursor: usize,
}

impl<'a> Iterator for PagesLineIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let s = self.pages.get_line(self.inner_cursor);
        self.inner_cursor += 1;
        s
    }
}

impl<'a> DoubleEndedIterator for PagesLineIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.inner_cursor == self.pages.lines_count() {
            return None;
        }

        let s = self
            .pages
            .get_line(self.pages.lines_count() - self.inner_cursor - 1);

        self.inner_cursor += 1;

        return s;
    }
}

#[derive(Default)]
pub struct Page {
    inner: String,
    indices: Vec<usize>,
}

// #[derive(Copy, Clone)]
// pub struct Segment {
//     start_position: u32,
//     terminal_width: u32,
// }

// #[derive(Debug)]
// pub struct StrSegment<'a> {
//     pub terminal_width: u32,
//     pub s: &'a str,
// }

impl Page {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: String::with_capacity(cap),
            indices: Vec::new(),
        }
    }

    pub fn add_str(&mut self, s: &str) {
        for line in s.lines() {
            self.add_line(line);
        }
    }

    pub fn add_line(&mut self, s: &str) -> usize {
        self.indices.push(self.inner.len());
        self.inner.push_str(s);
        self.len() - 1
    }

    pub fn add_str_only_if_in_cap(&mut self, s: &str) -> bool {
        if self.inner.len() + s.len() > self.inner.capacity() {
            return false;
        }
        self.add_str(s);
        return true;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn get_at(&self, idx: usize) -> Option<&str> {
        let start = *self.indices.get(idx)?;
        let end = *self.indices.get(idx + 1).unwrap_or(&self.inner.len());
        Some(&self.inner[start..end])
    }

    pub fn get_slice(&self, range: Range<usize>) -> Option<Vec<&str>> {
        let mut slice = Vec::with_capacity(range.len());
        for i in range {
            slice.push(self.get_at(i)?);
        }

        Some(slice)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.indices.clear();
    }
}

impl Index<usize> for Page {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_at(index).unwrap()
    }
}

pub struct PageLineIterator<'a> {
    page: &'a Page,
    idx: usize,
}

impl<'a> PageLineIterator<'a> {
    pub fn new(page: &'a Page) -> Self {
        Self { page, idx: 0 }
    }

    pub fn current_idx(&self) -> usize {
        self.idx.checked_sub(1).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.page.len()
    }
}

impl<'a> Iterator for PageLineIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.page.len() {
            return None;
        }
        let item = &self.page[self.idx];
        self.idx += 1;

        Some(item)
    }
}

impl<'a> DoubleEndedIterator for PageLineIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.idx == self.page.len() {
            return None;
        }
        let item = &self.page[self.page.len() - self.idx - 1];
        self.idx += 1;

        Some(item)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SearchResultLine<'a> {
    pub line: &'a str,
    pub line_index: usize,
    pub substr_start: usize,
}

pub struct PageSearchIterator<'a> {
    page_iter: PageLineIterator<'a>,
    search_str: &'a str,
}

impl<'a> PageSearchIterator<'a> {
    pub fn new(page: &'a Page, search_str: &'a str) -> Self {
        Self {
            page_iter: PageLineIterator::new(page),
            search_str,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PageSearchedLine {
    index: usize,
    substr_start: usize,
}

impl PageSearchedLine {
    pub fn new(index: usize, substr_start: usize) -> Self {
        Self {
            index,
            substr_start,
        }
    }

    pub fn as_str<'a, 'b>(&'b self, page: &'a Page) -> &'a str {
        &page[self.index]
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn substr_start(&self) -> usize {
        self.substr_start
    }
}

impl<'a> Iterator for PageSearchIterator<'a> {
    type Item = PageSearchedLine;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.page_iter.next() {
            if let Some(substr_start) = item.to_lowercase().find(self.search_str) {
                let line_index = self.page_iter.current_idx();
                // return Some(SearchResultLine {
                // line_index: self.page_iter.current_idx(),
                //     line: item,
                //     substr_start,
                // });

                return Some(PageSearchedLine::new(line_index, substr_start));
            }
        }

        return None;
    }
}
impl<'a> DoubleEndedIterator for PageSearchIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.page_iter.next_back() {
            if let Some(substr_start) = item.to_lowercase().find(self.search_str) {
                let line_index = self.page_iter.len() - self.page_iter.current_idx() - 1;
                // return Some(SearchResultLine {
                //     line_index: self.page_iter.len() - self.page_iter.current_idx() - 1,
                //     line: item,
                //     substr_start,
                // });

                return Some(PageSearchedLine::new(line_index, substr_start));
            }
        }
        return None;
    }
}
// #[test]
// fn test_pages() {
//     let mut pages = Pages::new(100, 30);
//     let mut buf = String::new();

//     for i in 0..1000 {
//         let s = i.to_string();
//         for _ in 0..1 {
//             buf += s.as_str();
//         }
//         pages.add_line(&buf);
//         buf.clear();
//     }

//     // for line in pages.get_lines() {
//     //     println!("{}", line);
//     // }

//     // println!("---------------");

//     // for line in pages.get_lines().rev() {
//     //     println!("{}", line);
//     // }

//     // println!("{:?}", pages.get_pages());
// }

#[test]
fn test_page_iterator() {
    let mut page = Page::new();
    page.add_line("hello world 0");
    page.add_line(" world hello world 1");
    page.add_line("world hello world 2");
    page.add_line("foo hello world 3");
    page.add_line("");
    page.add_line("");

    for line in PageLineIterator::new(&page).rev() {
        println!("{}", line);
    }

    let search_str = "hello";

    let iter = PageSearchIterator::new(&page, &search_str);
    for line in iter {
        println!("{:?}", line);
    }
    println!("=== reverse ===");
    let iter = PageSearchIterator::new(&page, &search_str).rev();
    for line in iter {
        println!("{:?}", line);
    }
    panic!("test exit");
}

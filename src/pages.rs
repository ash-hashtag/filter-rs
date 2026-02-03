use crate::command::Matcher;
use std::ops::{Index, Range};

pub struct Pages {
    pages: Vec<Page>,
    page_capacity: usize,
    global_offset: usize,
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
            global_offset: 0,
        }
    }

    pub fn add_line(&mut self, s: &str) {
        if self.pages.last_mut().unwrap().add_str_only_if_in_cap(s) {
            return;
        }

        if self.pages.len() == self.pages.capacity() {
            let mut page = self.pages.remove(0);
            self.global_offset += page.len();
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
            front_cursor: self.global_offset,
            back_cursor: self.lines_count(),
        }
    }

    pub fn first_index(&self) -> usize {
        self.global_offset
    }

    pub fn get_line(&self, idx: usize) -> Option<&str> {
        if idx < self.global_offset {
            return None;
        }

        let mut rdx = self.global_offset;

        for page in &self.pages {
            if rdx + page.len() > idx {
                return page.get_at(idx - rdx);
            }

            rdx += page.len();
        }

        return None;
    }

    pub fn lines_count(&self) -> usize {
        self.global_offset + self.current_lines_count()
    }

    pub fn current_lines_count(&self) -> usize {
        let mut size = 0;

        for page in &self.pages {
            size += page.len();
        }

        size
    }

    pub fn find_next<M: Matcher + ?Sized>(
        &self,
        matcher: &M,
        after_idx: usize,
    ) -> Option<(usize, std::ops::Range<usize>)> {
        let mut current_global_idx = self.global_offset;

        for page in &self.pages {
            let page_len = page.len();
            if current_global_idx + page_len > after_idx {
                let start_idx_in_page = if after_idx >= current_global_idx {
                    after_idx - current_global_idx + 1
                } else {
                    0
                };

                for i in start_idx_in_page..page_len {
                    if let Some(line) = page.get_at(i) {
                        if let Some(range) = matcher.is_match(line) {
                            return Some((current_global_idx + i, range));
                        }
                    }
                }
            }
            current_global_idx += page_len;
        }

        None
    }

    pub fn find_prev<M: Matcher + ?Sized>(
        &self,
        matcher: &M,
        before_idx: usize,
    ) -> Option<(usize, std::ops::Range<usize>)> {
        let mut current_global_idx = self.global_offset;
        let mut pages_with_start_indices = Vec::with_capacity(self.pages.len());

        for page in &self.pages {
            pages_with_start_indices.push((current_global_idx, page));
            current_global_idx += page.len();
        }

        for (start_idx, page) in pages_with_start_indices.into_iter().rev() {
            if start_idx < before_idx {
                let end_idx_in_page = (before_idx - start_idx).min(page.len());
                for i in (0..end_idx_in_page).rev() {
                    if let Some(line) = page.get_at(i) {
                        if let Some(range) = matcher.is_match(line) {
                            return Some((start_idx + i, range));
                        }
                    }
                }
            }
        }

        None
    }
}

pub struct PagesLineIterator<'a> {
    pages: &'a Pages,
    front_cursor: usize,
    back_cursor: usize,
}

impl<'a> Iterator for PagesLineIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_cursor >= self.back_cursor {
            return None;
        }
        let s = self.pages.get_line(self.front_cursor);
        self.front_cursor += 1;
        s
    }
}

impl<'a> DoubleEndedIterator for PagesLineIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front_cursor >= self.back_cursor {
            return None;
        }

        self.back_cursor -= 1;
        let s = self.pages.get_line(self.back_cursor);

        return s;
    }
}

#[derive(Default, Clone)]
pub struct Page {
    inner: String,
    indices: Vec<usize>,
}

const DEFAULT_PAGE_LINE_CAPACITY: usize = 8 * 1024;
const DEFAULT_PAGE_CAPACITY: usize = 64 * DEFAULT_PAGE_LINE_CAPACITY;

impl Page {
    pub fn new() -> Self {
        Self {
            inner: String::with_capacity(DEFAULT_PAGE_CAPACITY),
            indices: Vec::with_capacity(DEFAULT_PAGE_LINE_CAPACITY),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: String::with_capacity(cap),
            indices: Vec::new(),
        }
    }
    pub fn with_capacities(buffer_cap: usize, lines_cap: usize) -> Self {
        Self {
            inner: String::with_capacity(buffer_cap),
            indices: Vec::with_capacity(lines_cap),
        }
    }

    pub fn add_str(&mut self, s: &str) {
        for line in s.lines() {
            self.add_line(line);
        }
    }

    /// returns index of line
    pub fn add_line(&mut self, s: &str) -> usize {
        self.indices.push(self.inner.len());
        self.inner.push_str(s);
        self.len() - 1
    }

    fn add_str_only_if_in_cap(&mut self, s: &str) -> bool {
        if self.inner.len() + s.len() > self.inner.capacity() {
            return false;
        }
        self.add_str(s);
        return true;
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn get_at(&self, idx: usize) -> Option<&str> {
        let start = *self.indices.get(idx)?;
        let end = *self.indices.get(idx + 1).unwrap_or(&self.inner.len());
        Some(&self.inner[start..end])
    }

    pub fn get_slice(&self, range: Range<usize>) -> Option<Vec<&str>> {
        let allowed_range = 0..self.len();
        if !(allowed_range.contains(&range.start) && allowed_range.contains(&range.end)) {
            return None;
        }

        let mut slice = Vec::with_capacity(range.len());

        for i in range {
            slice.push(self.get_at(i).unwrap());
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

#[test]
fn test_page_iterator_forward_backward() {
    let mut page = Page::new();
    let lines = vec!["one", "two", "three"];
    for line in &lines {
        page.add_line(line);
    }

    // Forward
    let collected: Vec<&str> = PageLineIterator::new(&page).collect();
    assert_eq!(collected, lines);

    // Backward
    let collected_rev: Vec<&str> = PageLineIterator::new(&page).rev().collect();
    let mut lines_rev = lines.clone();
    lines_rev.reverse();
    assert_eq!(collected_rev, lines_rev);
}

#[test]
fn test_pages_iterator_forward_backward() {
    let mut pages = Pages::new(100, 5);
    let mut expected = Vec::new();

    // Add enough lines to fill a few pages roughly
    // Page cap is 100 bytes.
    // "line N" is 6 bytes. 100/6 ~= 16 lines per page.
    for i in 0..50 {
        let s = format!("line {}", i);
        pages.add_line(&s);
        expected.push(s);
    }

    // We need to compare specific strings, but expected owns the strings.
    // The iterator returns &str referencing the pages.
    // We can collect the iterator into Vec<&str> and compare matches.

    let collected: Vec<&str> = pages.get_lines_iter().collect();
    assert_eq!(collected.len(), 50);

    for (i, line) in collected.iter().enumerate() {
        assert_eq!(*line, expected[i].as_str());
    }

    // Reverse
    let collected_rev: Vec<&str> = pages.get_lines_iter().rev().collect();
    assert_eq!(collected_rev.len(), 50);

    let mut expected_rev = expected.clone();
    expected_rev.reverse();

    for (i, line) in collected_rev.iter().enumerate() {
        assert_eq!(*line, expected_rev[i].as_str());
    }
}

#[test]
fn test_page_search_iterator() {
    let mut page = Page::new();
    page.add_line("hello world"); // idx 0
    page.add_line("foo bar"); // idx 1
    page.add_line("world hello"); // idx 2
    page.add_line("baz"); // idx 3

    let search_str = "hello";
    let iter = PageSearchIterator::new(&page, search_str);

    let results: Vec<PageSearchedLine> = iter.collect();

    assert_eq!(results.len(), 2);

    // Check first match
    assert_eq!(results[0].index(), 0);
    // "hello" starts at 0 in "hello world"
    assert_eq!(results[0].substr_start(), 0);

    // Check second match
    assert_eq!(results[1].index(), 2);
    // "hello" starts at 6 in "world hello"
    assert_eq!(results[1].substr_start(), 6);

    // Reverse search
    let iter_rev = PageSearchIterator::new(&page, search_str).rev();
    let results_rev: Vec<PageSearchedLine> = iter_rev.collect();

    assert_eq!(results_rev.len(), 2);
    // Should be results reversed
    assert_eq!(results_rev[0].index(), 2);
    assert_eq!(results_rev[1].index(), 0);
}

#[test]
fn test_page_overflow() {
    let mut page = Page::with_capacity(10);
    // "hello" is 5 bytes.
    assert_eq!(page.add_str_only_if_in_cap("hello"), true);
    assert_eq!(page.add_str_only_if_in_cap("world"), true);
    // Capacity is 10, used 10. Next addition should fail.
    assert_eq!(page.add_str_only_if_in_cap("!"), false);

    assert_eq!(page.len(), 2);
    assert_eq!(&page[0], "hello");
    assert_eq!(&page[1], "world");
}

#[test]
fn test_pages_overflow_new_page() {
    // Capacity 10 per page, max 5 pages.
    let mut pages = Pages::new(10, 5);

    // Fill first page
    pages.add_line("0123456789"); // 10 bytes
    assert_eq!(pages.pages.len(), 1);

    // Trigger new page creation
    pages.add_line("next page");
    assert_eq!(pages.pages.len(), 2);

    assert_eq!(pages.get_line(0), Some("0123456789"));
    assert_eq!(pages.get_line(1), Some("next page"));
}

#[test]
fn test_pages_overflow_recycle() {
    // Capacity 10 per page, max 2 pages.
    let mut pages = Pages::new(10, 2);

    // Fill first page
    pages.add_line("page1-full"); // 10 bytes
    assert_eq!(pages.pages.len(), 1);

    // Fill second page (reached capacity of pages vector)
    pages.add_line("page2-full"); // 10 bytes
    assert_eq!(pages.pages.len(), 2);

    // Trigger recycle. Should remove first page (page1-full) and add new one.
    pages.add_line("page3-new");
    assert_eq!(pages.pages.len(), 2);

    // Expected lines: "page1-full" was index 0, but it was dropped.
    // "page2-full" was index 1 (relative to start of page 2).
    // Now page 1 is dropped, global_offset should be 1.
    // wait, "page1-full" in this test is ONE line but it fills the page capacity.
    assert_eq!(pages.global_offset, 1);
    assert_eq!(pages.lines_count(), 3);

    // Index 0 should be None now
    assert_eq!(pages.get_line(0), None);
    // Index 1 should be "page2-full"
    assert_eq!(pages.get_line(1), Some("page2-full"));
    // Index 2 should be "page3-new"
    assert_eq!(pages.get_line(2), Some("page3-new"));

    // Verify current_lines_count
    assert_eq!(pages.current_lines_count(), 2);
}

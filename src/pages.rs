use crate::command::Matcher;
use std::collections::VecDeque;
use std::ops::Index;

pub struct Pages {
    pages: VecDeque<Page>,
    page_capacity: usize,
    max_pages: usize,
    global_offset: usize,
}

impl Default for Pages {
    fn default() -> Self {
        Self::new(10_000, 10)
    }
}

impl Pages {
    pub fn new(page_capacity: usize, page_count: usize) -> Self {
        let mut pages = VecDeque::with_capacity(page_count);
        pages.push_back(Page::with_capacity(page_capacity));
        Self {
            page_capacity,
            max_pages: page_count,
            pages,
            global_offset: 0,
        }
    }

    pub fn add_line(&mut self, s: &str) {
        if self.pages.back_mut().unwrap().add_str_only_if_in_cap(s) {
            return;
        }

        if self.pages.len() == self.max_pages {
            let mut page = self.pages.pop_front().unwrap();
            self.global_offset += page.len();
            page.clear();
            page.add_str(s);
            self.pages.push_back(page);
        } else {
            let mut page = Page::with_capacity(self.page_capacity);
            page.add_str(s);
            self.pages.push_back(page);
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
        let skip = if after_idx >= self.global_offset {
            after_idx - self.global_offset + 1
        } else {
            0
        };

        let mut it = self.iter();
        it.fast_skip(skip);
        for (i, line) in it.enumerate() {
            if let Some(range) = matcher.is_match(line) {
                return Some((self.global_offset + skip + i, range));
            }
        }

        None
    }

    pub fn find_prev<M: Matcher + ?Sized>(
        &self,
        matcher: &M,
        before_idx: usize,
    ) -> Option<(usize, std::ops::Range<usize>)> {
        if before_idx <= self.global_offset {
            return None;
        }

        let total_count = self.lines_count();
        let skip_from_back = total_count.saturating_sub(before_idx);

        let mut it = self.iter();
        it.fast_skip_back(skip_from_back);
        for (i, line) in it.enumerate().rev() {
            if let Some(range) = matcher.is_match(line) {
                return Some((self.global_offset + i, range));
            }
        }

        None
    }

    pub fn find_all_matches<M: Matcher + ?Sized>(&self, matcher: &M) -> Vec<usize> {
        let mut matches = Vec::new();
        for (i, line) in self.iter().enumerate() {
            if matcher.is_match(line).is_some() {
                matches.push(self.global_offset + i);
            }
        }
        matches
    }

    pub fn iter(&self) -> PagesIter<'_> {
        PagesIter::new(self)
    }
}

pub struct PagesIter<'a> {
    pages: std::collections::vec_deque::Iter<'a, Page>,
    front_iter: Option<PageIter<'a>>,
    back_iter: Option<PageIter<'a>>,
    total_len: usize,
}

impl<'a> PagesIter<'a> {
    pub(crate) fn new(pages: &'a Pages) -> Self {
        Self {
            pages: pages.pages.iter(),
            front_iter: None,
            back_iter: None,
            total_len: pages.current_lines_count(),
        }
    }

    pub fn fast_skip(&mut self, mut n: usize) {
        while n > 0 {
            if let Some(iter) = &mut self.front_iter {
                let len = iter.len();
                if n < len {
                    iter.fast_skip(n);
                    self.total_len = self.total_len.saturating_sub(n);
                    return;
                }
                n -= len;
                self.total_len = self.total_len.saturating_sub(len);
                self.front_iter = None;
            } else if let Some(page) = self.pages.next() {
                self.front_iter = Some(page.iter());
            } else if let Some(iter) = &mut self.back_iter {
                let len = iter.len();
                let to_skip = n.min(len);
                iter.fast_skip(to_skip);
                self.total_len = self.total_len.saturating_sub(to_skip);
                return;
            } else {
                return;
            }
        }
    }

    pub fn fast_skip_back(&mut self, mut n: usize) {
        while n > 0 {
            if let Some(iter) = &mut self.back_iter {
                let len = iter.len();
                if n < len {
                    iter.fast_skip_back(n);
                    self.total_len = self.total_len.saturating_sub(n);
                    return;
                }
                n -= len;
                self.total_len = self.total_len.saturating_sub(len);
                self.back_iter = None;
            } else if let Some(page) = self.pages.next_back() {
                self.back_iter = Some(page.iter());
            } else if let Some(iter) = &mut self.front_iter {
                let len = iter.len();
                let to_skip = n.min(len);
                iter.fast_skip_back(to_skip);
                self.total_len = self.total_len.saturating_sub(to_skip);
                return;
            } else {
                return;
            }
        }
    }
}

impl<'a> Iterator for PagesIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.front_iter {
                if let Some(line) = iter.next() {
                    self.total_len = self.total_len.saturating_sub(1);
                    return Some(line);
                }
                self.front_iter = None;
            }
            if let Some(page) = self.pages.next() {
                self.front_iter = Some(page.iter());
            } else {
                return self.back_iter.as_mut()?.next().map(|line| {
                    self.total_len = self.total_len.saturating_sub(1);
                    line
                });
            }
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.fast_skip(n);
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.total_len, Some(self.total_len))
    }
}

impl<'a> DoubleEndedIterator for PagesIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(iter) = &mut self.back_iter {
                if let Some(line) = iter.next_back() {
                    self.total_len = self.total_len.saturating_sub(1);
                    return Some(line);
                }
                self.back_iter = None;
            }
            if let Some(page) = self.pages.next_back() {
                self.back_iter = Some(page.iter());
            } else {
                return self.front_iter.as_mut()?.next_back().map(|line| {
                    self.total_len = self.total_len.saturating_sub(1);
                    line
                });
            }
        }
    }
}

impl<'a> ExactSizeIterator for PagesIter<'a> {}

#[derive(Default, Clone)]
pub struct Page {
    inner: String,
    indices: Vec<usize>,
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

    pub fn clear(&mut self) {
        self.inner.clear();
        self.indices.clear();
    }

    pub fn iter(&self) -> PageIter<'_> {
        PageIter {
            page: self,
            front_idx: 0,
            back_idx: self.len(),
        }
    }
}

pub struct PageIter<'a> {
    page: &'a Page,
    front_idx: usize,
    back_idx: usize,
}

impl<'a> PageIter<'a> {
    pub fn fast_skip(&mut self, n: usize) {
        self.front_idx = (self.front_idx + n).min(self.back_idx);
    }

    pub fn fast_skip_back(&mut self, n: usize) {
        self.back_idx = self.back_idx.saturating_sub(n).max(self.front_idx);
    }
}

impl<'a> Iterator for PageIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.front_idx >= self.back_idx {
            return None;
        }
        let line = self.page.get_at(self.front_idx)?;
        self.front_idx += 1;
        Some(line)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.front_idx += n;
        self.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a> DoubleEndedIterator for PageIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front_idx >= self.back_idx {
            return None;
        }
        self.back_idx -= 1;
        self.page.get_at(self.back_idx)
    }
}

impl<'a> ExactSizeIterator for PageIter<'a> {
    fn len(&self) -> usize {
        self.back_idx.saturating_sub(self.front_idx)
    }
}

impl Index<usize> for Page {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.get_at(index).unwrap()
    }
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

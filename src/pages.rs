use crate::command::Matcher;
use std::ops::Index;

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

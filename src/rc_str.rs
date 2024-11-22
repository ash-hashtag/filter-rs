use std::{ops::Index, rc::Rc};

#[derive(Clone, Debug)]
pub struct RcStr {
    inner: Rc<str>,
    start: usize,
    end: usize,
}

impl RcStr {
    pub fn slice(&self, range: std::ops::Range<usize>) -> Self {
        return Self {
            inner: self.inner.clone(),
            start: range.start + self.start,
            end: range.end + self.start,
        };
    }
    pub fn checked_slice(&self, range: std::ops::Range<usize>) -> Option<Self> {
        if self.start + range.start <= self.end && self.start + range.end <= self.end {
            return Some(self.slice(range));
        }

        return None;
    }
}

impl AsRef<str> for RcStr {
    fn as_ref(&self) -> &str {
        &self.inner[self.start..self.end]
    }
}

impl From<String> for RcStr {
    fn from(value: String) -> Self {
        Self {
            end: value.len(),
            inner: Rc::from(value),
            start: 0,
        }
    }
}
impl From<&str> for RcStr {
    fn from(value: &str) -> Self {
        Self {
            end: value.len(),
            inner: Rc::from(value),
            start: 0,
        }
    }
}

impl std::fmt::Display for RcStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

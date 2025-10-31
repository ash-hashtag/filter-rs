use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

#[derive(Debug)]
pub struct Node<T> {
    next: Rc<RefCell<Option<Node<T>>>>,
    prev: Rc<RefCell<Option<Node<T>>>>,

    value: Rc<T>,
}

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self {
            next: self.next.clone(),
            prev: self.prev.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T> Node<T> {
    pub fn new(value: impl Into<Rc<T>>) -> Self {
        Self {
            next: Rc::new(RefCell::new(None)),
            prev: Rc::new(RefCell::new(None)),
            value: value.into(),
        }
    }
    pub fn next(&self) -> Option<Self> {
        if let Some(next) = self.next.borrow().as_ref() {
            return Some(next.clone());
        }

        None
    }
    pub fn prev(&self) -> Option<Self> {
        if let Some(prev) = self.prev.borrow().as_ref() {
            return Some(prev.clone());
        }

        None
    }

    pub fn value(&self) -> Rc<T> {
        self.value.clone()
    }

    pub fn push_next(&mut self, node: Self) {
        *self.next.borrow_mut() = Some(node);
    }

    pub fn pop_front(&mut self) {
        *self.prev.borrow_mut() = None;
    }
}

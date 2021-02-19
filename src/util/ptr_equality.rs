use std::{
    fmt,
    ops::Deref,
    rc::Rc,
};

use crate::sync::Arc;

/// A wrapper around an Arc/Rc/Box that changes equality to be based on the pointer contained inside.
///
/// This type is especially useful if a type doesn't implement Eq or PartialEq. Just Arc/Rc/Box the
/// type and then equality is based on the pointer.
pub struct PtrEquality<T> {
    data: T,
}

impl<T> PtrEquality<T> {
    /// Creates a new PtrEquality with the given data inside.
    /// This is only useful if T is an Arc/Rc/Box.
    pub fn new(data: T) -> Self {
        PtrEquality {
            data,
        }
    }
    /// Creates a new PtrEquality with a new Arc to the given data.
    pub fn new_arc(data: T) -> PtrEquality<Arc<T>> {
        PtrEquality {
            data: Arc::new(data),
        }
    }
    /// Creates a new PtrEquality with a new Rc to the given data.
    pub fn new_rc(data: T) -> PtrEquality<Rc<T>> {
        PtrEquality {
            data: Rc::new(data),
        }
    }
    /// Creates a new PtrEquality with a new Box to the given data.
    pub fn new_box(data: T) -> PtrEquality<Box<T>> {
        PtrEquality {
            data: Box::new(data),
        }
    }
}

impl<T> Deref for PtrEquality<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T: Clone> Clone for PtrEquality<T> {
    fn clone(&self) -> Self {
        Self::new(self.data.clone())
    }
}

impl<T: Default> Default for PtrEquality<T> {
    fn default() -> Self {
        PtrEquality::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for PtrEquality<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for PtrEquality<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

impl<T> Eq for PtrEquality<Arc<T>> {}
impl<T> PartialEq for PtrEquality<Arc<T>> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}
impl<T> From<T> for PtrEquality<Arc<T>> {
    fn from(data: T) -> Self {
        PtrEquality::new_arc(data)
    }
}

impl<T> Eq for PtrEquality<Rc<T>> {}
impl<T> PartialEq for PtrEquality<Rc<T>> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}
impl<T> From<T> for PtrEquality<Rc<T>> {
    fn from(data: T) -> Self {
        PtrEquality::new_rc(data)
    }
}

impl<T> Eq for PtrEquality<Box<T>> {}
impl<T> PartialEq for PtrEquality<Box<T>> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<T> From<T> for PtrEquality<Box<T>> {
    fn from(data: T) -> Self {
        PtrEquality::new_box(data)
    }
}

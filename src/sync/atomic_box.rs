// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    fmt,
    marker::PhantomData,
    mem::swap,
    ptr::{
        null_mut,
        NonNull,
    },
};

use crate::sync::{
    AtomicPtr,
    Ordering,
};

/// A container that owns a heap-allocated value that can be atomically loaded/set.
///
/// To make this type thread-safe, the range of atomic operations is limited to:
/// * loading the current value (which may be None)
/// * setting the value if it is None
///
/// Other operations (such as setting the value even if it's not None) requires
/// exclusive mutable access.
pub struct AtomicBox<T> {
    ptr: AtomicPtr<T>,
    _phantom: PhantomData<T>,
}

impl<T> AtomicBox<T> {
    /// Creates a new AtomicBox that contains the given Box.
    pub fn new(v: Box<T>) -> Self {
        // SAFETY: Given its a box, we know it was correctly allocated
        // and that we have exclusive control over the pointer.
        unsafe { Self::from_raw(Box::into_raw(v)) }
    }
    // Creates a new AtomicBox that contains the given data.
    pub fn new_box(data: T) -> Self {
        Self::new(Box::new(data))
    }
    /// Creates a new AtomicBox from the given raw pointer.
    ///
    /// # Safety
    /// AtomicBox must exclusively own this pointer and the pointer must have
    /// been allocated correctly.
    ///
    /// The pointer may be null, which creates an empty AtomicBox.
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        AtomicBox {
            ptr: AtomicPtr::new(raw),
            _phantom: PhantomData,
        }
    }
    /// Non-atomically gets the value within this AtomicBox as mutable.
    ///
    /// See [load](Self::load) for atomics.
    pub fn get(&mut self) -> Option<&mut T> {
        // SAFETY: The pointer is either null or an exclusive pointer to a value.
        return unsafe { self.ptr.get_mut().as_mut() };
    }
    /// Non-atomically gets the value within this AtomicBox if there is one.
    /// If there is no value in this AtomicBox, it will create a value using
    /// the given function.
    pub fn get_or_else<C>(&mut self, create: C) -> &mut T
    where C: FnOnce() -> Box<T> {
        // Sadly, this is necessary to get around limitations in the borrow checker.
        if self.get().is_some() {
            self.set(Some(create()));
        }
        // SAFETY: Either there was already a value to get *or* one was just set.
        unsafe { self.get().unwrap_unchecked() }
    }
    /// Non-atomically sets the value contained inside this AtomicBox.
    ///
    /// See [Self::set_if_none] if atomics are needed.
    pub fn set(&mut self, v: Option<Box<T>>) {
        let mut val_ptr = match v {
            Some(b) => Box::into_raw(b),
            None => null_mut(),
        };
        swap(&mut val_ptr, self.ptr.get_mut());
        if let Some(raw) = NonNull::new(val_ptr) {
            // SAFETY: We know this value is not null and that this object has exclusive ownership.
            // NOTE: If only safe functions are used, we know the value was correctly allocated.
            unsafe {
                Box::from_raw(raw.as_ptr());
            }
        }
    }
    /// Atomically loads a reference to the value in this AtomicBox.
    ///
    /// See [get](Self::get) for a non-atomic variant.
    pub fn load(&self) -> Option<&T> {
        // SAFETY: The pointer is either null or an exclusive pointer to a value.
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        return unsafe { self.ptr.load(Ordering::SeqCst).as_ref() };
    }
    /// Atomically loads the value contained within or attempts to set
    /// the value if there was None.
    ///
    /// Even if the create function is called, another thread may set
    /// the value before this one can. In that case, the newly created
    /// value will be discarded.
    pub fn load_or_else<C>(&self, create: C) -> &T
    where C: FnOnce() -> Box<T> {
        if let Some(value) = self.load() {
            value
        } else {
            self.set_if_none(create())
        }
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Box.
    ///
    /// If this AtomicBox already contains a value, the given Box will be dropped.
    /// If this AtomicBox was empty, it will now contained the given value box.
    ///
    /// This function returns a reference to the value contained.
    pub fn set_if_none(&self, val: Box<T>) -> &T {
        return match self.try_set_if_none(val) {
            Ok(current) => current,
            Err(current) => current,
        };
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Box.
    ///
    /// If this AtomicBox already contains a value, the given Box will be dropped.
    /// If this AtomicBox was empty, it will now contained the given value box.
    ///
    /// This function returns a result that indicates whether the swap
    /// occurred (Ok) or not (Err). Regardless, the value contained in the
    /// result is a reference to the current value of this AtomicBox.
    pub fn try_set_if_none(&self, val: Box<T>) -> Result<&T, &T> {
        let new_val = Box::into_raw(val);
        // OPTIMIZATION: Can different orderings work here?
        match self
            .ptr
            .compare_exchange(null_mut(), new_val, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => {
                // SAFETY: The exchange occurred so self now controls this pointer.
                Ok(unsafe { &*new_val })
            },
            Err(previous) => {
                // SAFETY: We just took this pointer out of a box. We need to put it back in one to be freed.
                unsafe {
                    Box::from_raw(new_val)
                };
                // SAFETY: The exchange did not occur, so this is the pointer owned by self.
                Err(unsafe { &*previous })
            },
        }
    }
}

impl<T> Default for AtomicBox<T> {
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Drop for AtomicBox<T> {
    fn drop(&mut self) {
        if let Some(raw) = NonNull::new(*self.ptr.get_mut()) {
            // SAFETY: We know the pointer is not null and that we have exclusive control over it.
            // NOTE: If only safe functions are used, we know the value was correctly allocated.
            unsafe {
                Box::from_raw(raw.as_ptr());
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for AtomicBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DropTester;

    #[test]
    fn atomic_box_same_size_as_box() {
        assert_eq!(
            std::mem::size_of::<AtomicBox<usize>>(),
            std::mem::size_of::<Box<usize>>()
        );
    }

    #[test]
    fn dropping_empty_box_works() {
        let _ = AtomicBox::<usize>::default();
    }

    #[test]
    fn dropping_occurs() {
        let mut flag = false;
        let _ = AtomicBox::new_box(DropTester::new(&mut flag));
        assert!(flag, "AtomicBox did not drop its value.");
    }

    #[test]
    fn new_boxes_from_values_works() {
        const TEST_VAL: usize = 10;
        let mut ab1 = AtomicBox::new_box(TEST_VAL);
        assert_eq!(*ab1.get().unwrap(), TEST_VAL);
        let mut ab2 = AtomicBox::new(Box::new(TEST_VAL));
        assert_eq!(*ab2.get().unwrap(), TEST_VAL);
    }

    #[test]
    fn from_raw_with_null_is_empty() {
        let mut ab1 = unsafe { AtomicBox::<usize>::from_raw(null_mut()) };
        assert!(ab1.get().is_none());
    }

    #[test]
    fn try_set_returns_ok_when_empty() {
        let ab = AtomicBox::<usize>::default();
        assert!(ab.try_set_if_none(Box::new(1)).is_ok());
    }
}

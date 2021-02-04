// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;
use std::marker::PhantomData;
use std::mem::swap;
use std::ptr::{
    null_mut,
    NonNull,
};

use crate::sync::{
    AtomicPtr,
    Ordering,
};

/// A container that owns a heap-allocated value that can be atomically loaded/set.
///
/// To make this type thread-safe, it supports a limited range of operations:
/// * loading the current value (which may be None)
/// * setting the value if it's None
///
/// Other operations are supported only with exclusive mutable access.
///
/// # Send and Sync
/// AtomicBox is only Send if T is Send. AtomicBox is only Sync if T is Sync.
pub struct AtomicBox<T> {
    ptr: AtomicPtr<T>,
    _phantom: PhantomData<T>,
}

impl<T> AtomicBox<T> {
    /// Creates a new AtomicBox that contains the given value.
    pub fn new(val: T) -> Self {
        Self::from_box(Box::new(val))
    }

    /// Creates a new AtomicBox from the given box.
    pub fn from_box(val: Box<T>) -> Self {
        // SAFETY: Given its a box, we know it was correctly allocated and that we have exclusive
        // control over the pointer.
        unsafe { Self::from_raw(Box::into_raw(val)) }
    }

    /// Creates a new AtomicBox from the given raw pointer.
    ///
    /// # Safety
    /// AtomicBox must exclusively own this pointer and the pointer must have been allocated
    /// correctly.
    ///
    /// The pointer may be null, which creates an empty box.
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        AtomicBox {
            ptr: AtomicPtr::new(raw),
            _phantom: PhantomData::default(),
        }
    }

    /// Creates an empty AtomicBox. Use set or set_if_none to set the value.
    pub fn empty() -> Self {
        AtomicBox {
            ptr: AtomicPtr::new(null_mut()),
            _phantom: PhantomData::default(),
        }
    }

    /// Gets the potential value within this box as mutable.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        // SAFETY: The pointer is either null or an exclusive pointer to a value.
        return unsafe { self.ptr.get_mut().as_mut() };
    }

    /// Sets the value contained in this box to another Box's value.
    ///
    /// This function requires exclusive mutability. If you want to set a shared AtomicBox,
    /// use set_if_none.
    pub fn set(&mut self, val: Box<T>) {
        let mut val_ptr = Box::into_raw(val);
        swap(&mut val_ptr, self.ptr.get_mut());
        if let Some(raw) = NonNull::new(val_ptr) {
            // SAFETY: We know this value is not null and that this object has exclusive ownership.
            // NOTE: If only safe functions are used, we know the value was correctly allocated.
            unsafe {
                Box::from_raw(raw.as_ptr());
            }
        }
    }

    /// Loads the potential value in this AtomicBox using the given ordering.
    ///
    /// If the box is empty, it will return None.
    pub fn load(&self, ordering: Ordering) -> Option<&T> {
        // SAFETY: The pointer is either null or an exclusive pointer to a value.
        return unsafe { self.ptr.load(ordering).as_ref() };
    }

    /// Uses a compare-and-exchange operation to attempt to set the value contained
    /// in the box.
    ///
    /// If the box already contains a value, the value given will be dropped.
    /// If the box was empty, the box will now contained the given value.
    ///
    /// This function returns a reference to the value contained in the box.
    pub fn set_if_none(&self, val: Box<T>, success: Ordering, failure: Ordering) -> &T {
        return match self.try_set_if_null(val, success, failure) {
            Ok(current) => current,
            Err(current) => current,
        };
    }

    /// Uses a compare-and-exchange operation to attempt to set the value contained
    /// in the box.
    ///
    /// If the box already contains a value, the value given will be dropped.
    /// If the box was empty, the box will now contained the given value.
    ///
    /// This function returns a result that indicates whether the swap occurred (Ok)
    /// or not (Err). Regardless, the value contained in the result is a reference
    /// to the current value contained in this box.
    pub fn try_set_if_null(
        &self,
        val: Box<T>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<&T, &T> {
        let new_val = Box::into_raw(val);
        match self.ptr.compare_exchange(null_mut(), new_val, success, failure) {
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
        self.load(Ordering::SeqCst).fmt(f)
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
        let _ = AtomicBox::<usize>::empty();
    }

    #[test]
    fn dropping_occurs() {
        let mut flag = false;
        let _ = AtomicBox::new(DropTester::new(&mut flag));
        assert!(flag, "AtomicBox did not drop its value.");
    }

    #[test]
    fn new_boxes_from_values_works() {
        const TEST_VAL: usize = 10;
        let mut ab1 = AtomicBox::new(TEST_VAL);
        assert_eq!(*ab1.as_mut().unwrap(), TEST_VAL);
        let mut ab2 = AtomicBox::from_box(Box::new(TEST_VAL));
        assert_eq!(*ab2.as_mut().unwrap(), TEST_VAL);
    }

    #[test]
    fn from_raw_with_null_is_empty() {
        let mut ab1 = unsafe { AtomicBox::<usize>::from_raw(null_mut()) };
        assert_eq!(ab1.as_mut(), None);
    }

    #[test]
    fn try_set_returns_ok_when_empty() {
        let ab = AtomicBox::<usize>::empty();
        assert!(
            ab.try_set_if_null(Box::new(1), Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        );
    }
}

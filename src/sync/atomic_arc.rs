// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::{
    null_mut,
    NonNull,
};

use crate::sync::{
    Arc,
    AtomicPtr,
    Ordering,
};

/// A container that holds an Arc that can be atomically loaded/set.
///
/// Arcs by themselves are 'atomic' in the sense one can clone them and share
/// the clone between threads. However, one can't change an Arc variable atomically.
///
/// To allow for an 'atomic' Arc field, we have to make a thread-safe type. This
/// limits the range of operations to:
/// * loading the current value (which may be None)
/// * setting the value if it's None
///
/// Other operations (such as setting the Arc even if it's not None) requires
/// exclusive mutable access.
pub struct AtomicArc<T> {
    ptr: AtomicPtr<T>,
    _phantom: PhantomData<Arc<T>>,
}
impl<T> AtomicArc<T> {
    /// Creates a new AtomicArc that contains the given Arc.
    pub fn new(val: Arc<T>) -> Self {
        // NOTE: into_raw consumes the Arc without decrementing.
        // AtomicArc now has 1 strong relationship with the Arc.
        AtomicArc {
            ptr: AtomicPtr::new(Arc::into_raw(val) as *mut T),
            _phantom: PhantomData,
        }
    }
    /// Creates a new AtomicArc that contains the given data.
    pub fn new_arc(data: T) -> Self {
        Self::new(Arc::new(data))
    }
    /// Creates an empty AtomicArc.
    pub fn empty() -> Self {
        AtomicArc {
            ptr: AtomicPtr::new(null_mut()),
            _phantom: PhantomData,
        }
    }
    /// Gets the potential value using mutability.
    pub fn get(&mut self) -> Option<&T> {
        // SAFETY: This struct keeps the reference count at 1 or more, so it won't be freed.
        unsafe { self.ptr.get_mut().as_ref() }
    }
    /// Potentially returns a clone of the Arc using mutability.
    pub fn get_arc(&mut self) -> Option<Arc<T>> {
        let ptr = NonNull::new(*self.ptr.get_mut().deref())?;
        // SAFETY: We now the ptr is the result of Arc::into_raw and the ptr is stored in self.ptr
        unsafe { Some(Self::increment_and_make_arc(ptr)) }
    }
    /// Sets the value contained to be another Arc.
    ///
    /// This function requires exclusive mutability. If you want to set a
    /// shared AtomicArc, use set_if_none.
    pub fn set(&mut self, val: Arc<T>) {
        // SAFETY: We hold a reference count and are getting rid of it.
        if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
            unsafe { Arc::decr_strong_count(ptr.as_ptr()) }
        }
        // We consume the Arc and take its place
        *self.ptr.get_mut() = Arc::into_raw(val) as *mut T;
    }
    /// Loads the potential value in the Arc using the given ordering.
    ///
    /// If there is no Arc, it will return None.
    pub fn load(&self, order: Ordering) -> Option<&T> {
        let ptr = self.load_ptr(order)?;
        // SAFETY: This struct keeps the reference count at 1 or more, so it won't be freed.
        Some(unsafe { &*ptr.as_ptr() })
    }
    /// Loads and creates a clone of the Arc using the given ordering.
    ///
    /// If there is no Arc, it will return None.
    pub fn load_arc(&self, order: Ordering) -> Option<Arc<T>> {
        let ptr = self.load_ptr(order)?;
        // SAFETY: We now the ptr is the result of Arc::into_raw and the ptr is stored in self.ptr
        Some(unsafe { Self::increment_and_make_arc(ptr) })
    }
    /// Loads the internal pointer that represents the Arc.
    /// This pointer should be from [Arc::into_raw].
    fn load_ptr(&self, order: Ordering) -> Option<NonNull<T>> {
        NonNull::new(self.ptr.load(order))
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Arc.
    ///
    /// If self already contains a value, the Arc will be dropped.
    /// If self did not contain a value, it will now contain the given Arc.
    ///
    /// This function returns a reference to the value contained.
    pub fn set_if_none(&self, to: Arc<T>, success: Ordering, failure: Ordering) -> &T {
        match self.try_set_if_none(to, success, failure) {
            Ok(v) => v,
            Err(v) => v,
        }
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Arc.
    ///
    /// If self already contains a value, the Arc will be dropped.
    /// If self did not contain a value, it will now contain the given Arc.
    ///
    /// This function returns a result that indicates whether the swap
    /// occurred (Ok) or not (Err). Regardless, the value contained in the
    /// result is a reference to the current value.
    pub fn try_set_if_none(
        &self,
        to: Arc<T>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<&T, &T> {
        let raw_new_val = Arc::as_ptr(&to) as *mut T;
        match self.ptr.compare_exchange(null_mut(), raw_new_val, success, failure) {
            Ok(ptr) => {
                // NOTE: We have to forget the old Arc since it's ptr is now in self.
                std::mem::forget(to);
                // SAFETY: This ptr is now owned by this type and is not null.
                Ok(unsafe { &*ptr })
            },
            // SAFETY: This ptr is owned by this type and not null.
            // The Arc they tried to set will drop itself.
            Err(ptr) => Err(unsafe { &*ptr }),
        }
    }

    /// Increments the strong count of the ptr and then creates a new Arc.
    /// # Safety
    /// 1. The ptr must be from [Arc::into_raw].
    /// 2. The ptr must *also* still be stored in self.ptr.
    unsafe fn increment_and_make_arc(ptr: NonNull<T>) -> Arc<T> {
        // SAFETY: The value must be from Arc::into_raw
        Arc::incr_strong_count(ptr.as_ptr());
        // SAFETY: We've incremented the count so self and this new arc can co-exist.
        Arc::from_raw(ptr.as_ptr())
    }
}
impl<T> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
            // SAFETY: This struct owns a reference count.
            unsafe { Arc::decr_strong_count(ptr.as_ptr()) }
        }
    }
}
impl<T: fmt::Debug> fmt::Debug for AtomicArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load(Ordering::SeqCst).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DropTester;

    #[test]
    fn atomic_arc_same_size_as_arc() {
        assert_eq!(
            std::mem::size_of::<AtomicArc<usize>>(),
            std::mem::size_of::<Arc<usize>>()
        );
    }

    #[test]
    fn dropping_empty_atomic_arc_works() {
        let _ = AtomicArc::<usize>::empty();
    }

    #[test]
    fn dropping_occurs() {
        let mut flag = false;
        let _ = AtomicArc::new_arc(DropTester::new(&mut flag));
        assert!(flag, "AtomicArc did not drop its value.");
    }

    #[test]
    fn new_arcs_from_values_works() {
        const TEST_VAL: usize = 10;
        let mut aa1 = AtomicArc::new(Arc::new(TEST_VAL));
        assert_eq!(*aa1.get().unwrap(), TEST_VAL);
        let mut aa1 = AtomicArc::new_arc(TEST_VAL);
        assert_eq!(*aa1.get().unwrap(), TEST_VAL);
    }

    #[test]
    fn try_set_returns_ok_when_empty() {
        let aa = AtomicArc::<usize>::empty();
        assert!(
            aa.try_set_if_none(Arc::new(1), Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        );
    }
}

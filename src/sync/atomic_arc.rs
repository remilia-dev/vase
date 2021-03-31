// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    fmt,
    marker::PhantomData,
    ops::Deref,
    ptr::{
        null_mut,
        NonNull,
    },
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
/// * setting the value if it is None
///
/// Other operations (such as setting the Arc even if it's not None) requires
/// exclusive mutable access.
pub struct AtomicArc<T> {
    ptr: AtomicPtr<T>,
    _phantom: PhantomData<Arc<T>>,
}

impl<T> AtomicArc<T> {
    /// Creates a new AtomicArc that contains the given Arc.
    pub fn new(v: Arc<T>) -> Self {
        // NOTE: into_raw consumes the Arc without decrementing the strong count.
        // AtomicArc now 'owns' that 1 strong relationship with the Arc.
        AtomicArc {
            ptr: AtomicPtr::new(Arc::into_raw(v) as *mut T),
            _phantom: PhantomData,
        }
    }
    /// Creates a new AtomicArc that contains the given data.
    pub fn new_arc(data: T) -> Self {
        Self::new(Arc::new(data))
    }
    /// Creates a new AtomicArc from the given raw pointer.
    ///
    /// # Safety
    /// AtomicArc is responsible for decrementing the strong count by one
    /// when dropped. The pointer must be from [Arc::into_raw].
    ///
    /// This pointer may be null, which creates an empty AtomicArc.
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        AtomicArc {
            ptr: AtomicPtr::new(raw),
            _phantom: PhantomData,
        }
    }
    /// Non-atomically gets the value within this AtomicArc.
    ///
    /// See [load](Self::load) for atomics.
    pub fn get(&mut self) -> Option<&T> {
        // SAFETY: This struct keeps the reference count at 1 or more, so it won't be freed.
        unsafe { self.ptr.get_mut().as_ref() }
    }
    /// Non-atomically gets and clones the Arc contained within this AtomicArc.
    ///
    /// See [load_arc](Self::load_arc) for atomics.
    pub fn get_arc(&mut self) -> Option<Arc<T>> {
        let ptr = NonNull::new(*self.ptr.get_mut().deref())?;
        // SAFETY: We now the ptr is the result of Arc::into_raw.
        unsafe { Some(Self::increment_and_make_arc(ptr)) }
    }
    /// Non-atomically gets the value within this AtomicArc if there is one.
    /// If there is no value in this AtomicArc, it will create a value using
    /// the given function.
    pub fn get_or_else<C>(&mut self, create: C) -> &T
    where C: FnOnce() -> Arc<T> {
        if self.get().is_none() {
            self.set(Some(create()));
        }
        // SAFETY: Either there was already a value to get *or* one was just set.
        unsafe { self.get().unwrap_unchecked() }
    }
    /// Non-atomically sets the value contained inside this AtomicArc.
    ///
    /// See [Self::set_if_none] if atomics are needed.
    pub fn set(&mut self, v: Option<Arc<T>>) {
        // SAFETY: We hold a reference count and are getting rid of it.
        if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
            unsafe { Arc::decrement_strong_count(ptr.as_ptr()) }
        }
        *self.ptr.get_mut() = match v {
            Some(val) => Arc::into_raw(val) as *mut T,
            None => null_mut(),
        };
    }
    /// Atomically loads a reference to the value in this AtomicArc.
    ///
    /// See [get](Self::get) for a non-atomic variant.
    pub fn load(&self) -> Option<&T> {
        // SAFETY: This struct keeps the reference count at 1 or more, so it won't be freed.
        Some(unsafe { self.load_ptr()?.as_ref() })
    }
    /// Atomically loads and clones the Arc contained within this AtomicArc.
    ///
    /// See [get_arc](Self::get_arc) for a non-atomic variant.
    pub fn load_arc(&self) -> Option<Arc<T>> {
        // SAFETY: We now the ptr is the result of Arc::into_raw.
        Some(unsafe { Self::increment_and_make_arc(self.load_ptr()?) })
    }
    /// Atomically loads the value contained within or attempts to set
    /// the value if there was None.
    ///
    /// Even if the create function is called, another thread may set
    /// the value before this one can. In that case, the newly created
    /// value will be discarded.
    pub fn load_or_else<C>(&self, create: C) -> Arc<T>
    where C: FnOnce() -> Arc<T> {
        let ptr = self.load_ptr().unwrap_or_else(|| {
            let new_value = create();
            let new_ptr = Arc::as_ptr(&new_value) as *mut T;
            match self
                .ptr
                .compare_exchange(null_mut(), new_ptr, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => {
                    std::mem::forget(new_value);
                    // SAFETY: new_ptr is from an allocated Arc, so it can't be null.
                    unsafe { NonNull::new_unchecked(new_ptr) }
                },
                Err(ptr) => unsafe {
                    // SAFETY: Since the compare-and-exchange failed, this ptr can't be null.
                    NonNull::new_unchecked(ptr)
                },
            }
        });
        // SAFETY: Either this pointer is from this object (which should then be an Arc ptr)
        // or it is from a freshly created Arc (in which case it was stored in self.ptr).
        unsafe { AtomicArc::increment_and_make_arc(ptr) }
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Arc.
    ///
    /// If this AtomicArc already contains a value, the given Arc will be dropped.
    /// If this AtomicArc was empty, it will now contain the given Arc.
    ///
    /// This function returns a reference to the value contained.
    pub fn set_if_none(&self, to: Arc<T>) -> &T {
        match self.try_set_if_none(to) {
            Ok(v) => v,
            Err(v) => v,
        }
    }
    /// Uses a compare-and-exchange operation to attempt to set the value
    /// to the given Arc.
    ///
    /// If this AtomicArc already contains a value, the given Arc will be dropped.
    /// If this AtomicArc was empty, it will now contain the given Arc.
    ///
    /// This function returns a result that indicates whether the swap
    /// occurred (Ok) or not (Err). Regardless, the value contained in the
    /// result is a reference to the current value of this AtomicArc.
    pub fn try_set_if_none(&self, to: Arc<T>) -> Result<&T, &T> {
        let raw_new_val = Arc::as_ptr(&to) as *mut T;
        // OPTIMIZATION: Can different orderings work here?
        match self.ptr.compare_exchange(
            null_mut(),
            raw_new_val,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
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
    /// Loads the internal pointer that represents the Arc.
    /// This pointer should be from [Arc::into_raw].
    fn load_ptr(&self) -> Option<NonNull<T>> {
        // OPTIMIZATION: Could we use Ordering::Acquire here?
        NonNull::new(self.ptr.load(Ordering::SeqCst))
    }
    /// Increments the strong count of the ptr and then creates a new Arc.
    /// # Safety
    /// 1. The ptr must be from [Arc::into_raw].
    /// 2. The ptr must *also* still be stored in self.ptr.
    unsafe fn increment_and_make_arc(ptr: NonNull<T>) -> Arc<T> {
        // SAFETY: The value must be from Arc::into_raw
        Arc::increment_strong_count(ptr.as_ptr());
        // SAFETY: We've incremented the count so self and this new arc can co-exist.
        Arc::from_raw(ptr.as_ptr())
    }
}

impl<T> Default for AtomicArc<T> {
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        if let Some(ptr) = NonNull::new(*self.ptr.get_mut()) {
            // SAFETY: This struct owns a reference count.
            unsafe { Arc::decrement_strong_count(ptr.as_ptr()) }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for AtomicArc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.load().fmt(f)
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
        let _ = AtomicArc::<usize>::default();
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
    fn from_raw_with_null_is_empty() {
        let mut aa = unsafe { AtomicArc::<usize>::from_raw(null_mut()) };
        assert!(aa.get().is_none())
    }

    #[test]
    fn try_set_returns_ok_when_empty() {
        let aa = AtomicArc::<usize>::default();
        assert!(aa.try_set_if_none(Arc::new(1)).is_ok());
    }
}

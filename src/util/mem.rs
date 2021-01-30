// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::ptr::NonNull;

/// Allocates the given data on the heap by making a box and returning the pointer inside.
///
/// The pointer returned is the only pointer to this data.
/// # Note
/// You'll want to make sure you free the data with [free].
/// # Panics
/// Panics if the value you give is a zero-sized type.
#[must_use]
pub fn alloc<T>(data: T) -> NonNull<T> {
    // Sadly, Rust's generics are quite limited in what they can do at compile-time.
    // As such, what should be a compile-time check is now a runtime check.
    if std::mem::size_of::<T>() == 0 {
        panic!("Cannot use zero-sized types with alloc!")
    }
    let new_ptr = Box::into_raw(Box::new(data));
    // SAFETY: This function does not support zero-sized types, so there should be a pointer.
    unsafe { NonNull::new_unchecked(new_ptr) }
}

/// Frees the given NonNull pointer by putting the pointer back into a box.
/// # Safety
/// Only pointers returned from [alloc] should be passed to this function.
/// This should also be the last reference to this pointer. Any reads/writes to this pointer
/// beyond this function will cause undefined behavior.
pub unsafe fn free<T>(ptr: NonNull<T>) {
    drop(Box::from_raw(ptr.as_ptr()));
}

/// Initializes a static-length array by calling a function for every value.
///
/// This utility is only meant to initialize an array with every item being
/// the same value. As such, no index is given to the function.
///
/// # Replacement
/// If both the inline_const feature and the const_in_array_repeat_expressions feature
/// are stabilized, this function *could* be replaced by them. Currently, however, they
/// have a problem in that they can't handle generics.
pub fn make_static_array<T, const LENGTH: usize>(val_fn: &dyn Fn() -> T) -> [T; LENGTH] {
    use std::mem::MaybeUninit;
    let mut array: [MaybeUninit<T>; LENGTH] = MaybeUninit::uninit_array();
    for array_val in &mut array {
        *array_val = MaybeUninit::new(val_fn());
    }
    // SAFETY: We know from above that every element of the array has been initialized.
    unsafe { MaybeUninit::array_assume_init(array) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DropTester;

    #[test]
    fn allocate_works_properly() {
        let ptr = alloc(10);
        unsafe {
            assert_eq!(*ptr.as_ref(), 10);
            free(ptr);
        }
    }

    #[test]
    #[should_panic]
    fn allocate_panics_on_zero_sized_type() {
        let _ = alloc(());
    }

    #[test]
    fn free_runs_drop() {
        let mut flag = false;
        let ptr = alloc(DropTester::new(&mut flag));
        unsafe {
            free(ptr);
        }

        assert!(flag, "free did not properly drop the pointer.");
    }

    #[test]
    fn make_static_array_returns_filled_array() {
        let made = make_static_array::<_, 5>(&|| 254u8);
        let manual = [254, 254, 254, 254, 254];
        assert_eq!(made, manual);
    }

    #[test]
    fn make_static_array_handles_zero_sized_array() {
        let made = make_static_array::<_, 2>(&|| ());
        let manual = [(), ()];
        assert_eq!(made, manual);
    }
}

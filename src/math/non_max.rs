// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    fmt,
    num::NonZeroU32,
};

use crate::util::Conversions;

/// A u32 that can't be it's maximum value.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NonMaxU32(NonZeroU32);

impl NonMaxU32 {
    /// The maximum possible NonMaxU32.
    pub const MAX: NonMaxU32 = unsafe { NonMaxU32::new_unchecked(u32::MAX - 1) };
    /// Creates a non-max if the given value is not [u32::MAX].
    pub fn new(n: u32) -> Option<NonMaxU32> {
        Some(NonMaxU32(NonZeroU32::new(!n)?))
    }
    /// Creates a non-max u32 if the given value is not greater than or equal to [u32::MAX].
    pub fn new_usize(n: usize) -> Option<NonMaxU32> {
        Self::new(n.try_into::<u32>().ok()?)
    }
    /// Creates a non-max without checking the value.
    /// # Safety
    /// The provided value must not be [u32::MAX].
    pub const unsafe fn new_unchecked(n: u32) -> NonMaxU32 {
        NonMaxU32(NonZeroU32::new_unchecked(!n))
    }
    /// Returns the value as a primitive type.
    pub fn get(self) -> u32 {
        !self.0.get()
    }
    /// Increments the value stored inside.
    /// # Panics
    /// Panics if the value is incremented to [u32::MAX].
    pub fn increment(&mut self) {
        let new_value = Self::new(self.get() + 1);
        *self = new_value.expect("NonMaxU32 was incremented beyond its maximum value.");
    }
}

impl fmt::Debug for NonMaxU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NonMaxU32({})", !self.0.get())
    }
}

impl fmt::Display for NonMaxU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (!self.0.get()).fmt(f)
    }
}

impl From<u16> for NonMaxU32 {
    fn from(v: u16) -> Self {
        // SAFETY: Since v is a u16, we know it is not the maximum value of a u32.
        unsafe { NonMaxU32::new_unchecked(v as u32) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_size_is_same() {
        use std::mem::size_of;
        assert_eq!(size_of::<Option<NonMaxU32>>(), size_of::<NonMaxU32>());
    }

    #[test]
    fn new_returns_non_on_u32_max() {
        assert!(NonMaxU32::new(u32::MAX).is_none());
    }

    #[test]
    fn new_creates_correct_value() {
        const TEST_CASE: u32 = 100;
        let value = NonMaxU32::new(TEST_CASE).unwrap();
        assert_eq!(value.get(), TEST_CASE);
    }

    #[test]
    fn can_into_a_u16() {
        const TEST_CASE: u16 = 100;
        let value: NonMaxU32 = TEST_CASE.into();
        assert_eq!(value, NonMaxU32::new(TEST_CASE as u32).unwrap());
    }
}

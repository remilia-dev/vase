// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    fmt,
    num::NonZeroU32,
};

/// A u32 that can't be it's maximum value.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NonMaxU32 {
    value: NonZeroU32,
}

impl NonMaxU32 {
    /// The maximum possible NonMaxU32.
    pub const MAX: NonMaxU32 = unsafe { NonMaxU32::new_unchecked(u32::MAX - 1) };
    /// Creates a non-max if the given value is not [u32::MAX].
    pub fn new(n: u32) -> Option<NonMaxU32> {
        Some(NonMaxU32 { value: NonZeroU32::new(!n)? })
    }
    /// Creates a non-max without checking the value.
    /// # Safety
    /// The provided value must not be [u32::MAX].
    pub const unsafe fn new_unchecked(n: u32) -> NonMaxU32 {
        NonMaxU32 {
            value: NonZeroU32::new_unchecked(!n),
        }
    }
    /// Returns the value as a primitive type.
    pub fn get(self) -> u32 {
        !self.value.get()
    }
}

impl fmt::Display for NonMaxU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}

impl From<u16> for NonMaxU32 {
    fn from(v: u16) -> Self {
        // SAFETY: Since v is a u16, we know it is not the maximum value of a u32.
        unsafe { NonMaxU32::new_unchecked(v as u32) }
    }
}
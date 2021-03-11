// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    fmt,
    ops::{
        Deref,
        Index,
        IndexMut,
    },
};

use crate::math::NonMaxU32;

/// A vector that is indexable by u32 and NonMaxU32.
///
/// This type is mostly the same as a [Vec], except that its length is
/// limited to u32::MAX - 1. It can also be indexed by [u32] and [NonMaxU32].
#[derive(Clone)]
pub struct Vec32<T>(Vec<T>);

impl<T> Vec32<T> {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: NonMaxU32) -> Self {
        Self(Vec::with_capacity(capacity.get() as usize))
    }

    pub fn reserve(&mut self, additional: u32) {
        self.0.reserve(additional as usize);
    }

    pub fn swap_remove(&mut self, index: u32) -> T {
        self.0.swap_remove(index as usize)
    }

    pub fn insert(&mut self, index: NonMaxU32, element: T) {
        self.0.insert(index.get() as usize, element);
        self.check_size();
    }

    pub fn remove(&mut self, index: u32) -> T {
        self.0.remove(index as usize)
    }

    pub fn push(&mut self, value: T) {
        self.0.push(value);
        self.check_size();
    }

    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn len(&self) -> NonMaxU32 {
        // SAFETY: Only Deref (not DerefMut) is implemented. All mutable functions
        // that can possibly increase the internal Vec's size call check_size().
        // check_size() ensures that a panic occurs if the length is too large.
        unsafe { NonMaxU32::new_unchecked(self.0.len() as u32) }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: u32) -> Option<&T> {
        self.0.get(index as usize)
    }

    pub fn get_mut(&mut self, index: u32) -> Option<&mut T> {
        self.0.get_mut(index as usize)
    }

    fn check_size(&mut self) {
        if self.0.len() >= u32::MAX as usize {
            self.clear();
            panic!("Vec32 has more than u32::MAX - 1 items.");
        }
    }
}

impl<T> Deref for Vec32<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Index<u32> for Vec32<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl<T> IndexMut<u32> for Vec32<T> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl<T> Index<NonMaxU32> for Vec32<T> {
    type Output = T;

    fn index(&self, index: NonMaxU32) -> &Self::Output {
        &self.0[index.get() as usize]
    }
}

impl<T> IndexMut<NonMaxU32> for Vec32<T> {
    fn index_mut(&mut self, index: NonMaxU32) -> &mut Self::Output {
        &mut self.0[index.get() as usize]
    }
}

impl<T: fmt::Debug> fmt::Debug for Vec32<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

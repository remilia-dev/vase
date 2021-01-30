// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

// Following features allow some unsafe to be removed in StringCache
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]
#![feature(arc_mutate_strong_count)]

pub mod c;
pub mod sync;
#[cfg(test)]
pub mod test_utils;
pub mod util;

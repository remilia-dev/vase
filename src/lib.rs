// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

// Following features allow some unsafe to be removed in StringCache
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]
// Allows for some cleaner match code
#![feature(destructuring_assignment)]
// Required for sensible generics
#![feature(trait_alias)]
// Allows cleaner atomic code
#![feature(option_result_unwrap_unchecked)]
// The following warnings have been added since they work with the project and have good justifications.
#![warn(clippy::cognitive_complexity)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::explicit_iter_loop)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::match_wildcard_for_single_variants)]
#![warn(clippy::pattern_type_mismatch)]
#![warn(clippy::ptr_as_ptr)]
#![warn(clippy::too_many_lines)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::useless_let_if_seq)]

pub mod c;
pub mod error;
pub mod math;
pub mod sync;
#[cfg(test)]
pub mod test_utils;
pub mod util;

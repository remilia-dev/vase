// Following features allow some unsafe to be removed in StringCache
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]

pub mod c;
pub mod sync;
pub mod util;

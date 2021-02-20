// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use vase_macros::enum_with_properties;

pub use char_ext::*;
pub use ptr_equality::PtrEquality;
pub use string_builder::StringBuilder;
pub use string_cache::{
    CachedString,
    CachedStringData,
    StringCache,
};
pub use vase_macros::enum_with_properties;
/// Memory utilities.
pub mod mem;

mod char_ext;
mod ptr_equality;
mod string_builder;
mod string_cache;

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use char_ext::*;
pub use source_location::SourceLocation;
pub use string_builder::StringBuilder;
pub use string_cache::{
    CachedString,
    CachedStringData,
    StringCache,
};
/// Memory utilities.
pub mod mem;

mod char_ext;
mod source_location;
mod string_builder;
mod string_cache;

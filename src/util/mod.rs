// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use char_ext::{
    CharExt,
    DecodedChar,
    Utf8DecodeError,
};
pub use num_parsing::{
    parse_int,
    parse_real,
    NumBase,
    ParsableInt,
    ParsableReal,
    ParseNumberError,
    ParsedNumber,
};
pub use ptr_equality::PtrEquality;
pub use severity::Severity;
pub use source_location::{
    FileId,
    SourceLocation,
};
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
mod num_parsing;
mod ptr_equality;
mod severity;
mod source_location;
mod string_builder;
mod string_cache;

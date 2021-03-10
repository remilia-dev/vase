// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use base::NumBase;
pub use non_max::NonMaxU32;
pub use num::{
    Integer,
    Number,
    Real,
};
pub use parsing::{
    NumberResult,
    ParseNumberError,
    ParsedNumber,
};
pub use sign::Sign;

mod base;
mod non_max;
mod num;
mod parsing;
mod sign;

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use error::{
    ParseError,
    ParseErrorKind,
};
pub use implementation::Parser;

mod error;
mod implementation;

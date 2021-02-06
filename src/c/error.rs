// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    sync::Arc,
    util::Utf8DecodeError,
};

#[derive(Clone, Debug)]
pub enum CError {
    // Lexer errors
    Utf8DecodeError(Utf8DecodeError),
    IOError(Arc<std::io::Error>),
}

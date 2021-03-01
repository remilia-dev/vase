// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

/// An error value that signals how far back the error should go in a chain
/// of functions. The error that caused this to occur has been reported elsewhere.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ErrorScope {
    /// A block error is an error that makes the rest of the block unreadable.
    Block,
    /// A fatal error should go all the way up the function chain as it is unrecoverable.
    Fatal,
}

impl ErrorScope {
    pub fn is_block(self) -> bool {
        matches!(self, Self::Block)
    }

    pub fn is_fatal(self) -> bool {
        matches!(self, Self::Fatal)
    }
}

pub type ResultScope<T> = Result<T, ErrorScope>;

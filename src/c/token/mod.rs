// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

pub use self::{
    include_type::IncludeType,
    keyword::Keyword,
    kind::TokenKind,
    string_enc::StringEnc,
};
use crate::util::{
    FileId,
    SourceLoc,
};

mod include_type;
mod keyword;
mod kind;
mod string_enc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    loc: SourceLoc,
    whitespace_before: bool,
    kind: TokenKind,
}

impl Token {
    pub fn new(loc: SourceLoc, whitespace_before: bool, kind: TokenKind) -> Token {
        Token { loc, whitespace_before, kind }
    }

    pub fn new_first_byte(file_id: FileId, kind: TokenKind) -> Token {
        Token {
            loc: SourceLoc::new_first_byte(file_id),
            whitespace_before: false,
            kind,
        }
    }

    pub fn loc(&self) -> SourceLoc {
        self.loc
    }
    pub fn whitespace_before(&self) -> bool {
        self.whitespace_before
    }
    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }
    pub fn kind_mut(&mut self) -> &mut TokenKind {
        &mut self.kind
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_token_is_at_most_32_bytes() {
        // Testing limits the size of CToken since even small size increases will result in
        // higher memory usage (and not by a tiny amount).
        let size = std::mem::size_of::<Token>();
        assert!(
            size <= 32,
            "CToken is {} bytes when it should be 32 or less.",
            size
        );
    }
}

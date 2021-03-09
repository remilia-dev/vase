// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::TokenKind,
    error::{
        CodedError,
        Severity,
    },
    sync::Arc,
    util::{
        enum_with_properties,
        SourceLoc,
        Utf8DecodeError,
    },
};

#[derive(Clone, Debug)]
pub struct LexerError {
    pub kind: LexerErrorKind,
    pub loc: SourceLoc,
}

impl CodedError for LexerError {
    fn severity(&self) -> Severity {
        self.kind.severity()
    }

    fn code_number(&self) -> u32 {
        self.kind.code_number()
    }

    fn code_prefix(&self) -> &'static str {
        self.kind.code_prefix()
    }

    fn message(&self) -> String {
        self.kind.message()
    }
}

enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum LexerErrorKind {
        // == Fatals
        #[values(Fatal, 800)]
        Utf8Decode(Utf8DecodeError),
        #[values(Fatal, 801)]
        Io(Arc<std::io::Error>),
        // == Errors
        #[values(Error, 500)]
        MissingCorrespondingIf(TokenKind),
        #[values(Error, 501)]
        MissingCorrespondingEndIf(TokenKind),
        #[values(Error, 510)]
        UnendedComment,
        #[values(Error, 511)]
        UnendedInclude(bool),
        #[values(Error, 512)]
        UnendedString(bool),
        // NOTE: Error codes 600-610 and warning codes 300-310 are reserved for literals
    }

    impl CodedError for LexerErrorKind {
        #[property]
        fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        fn code_number(&self) -> u32 {}

        fn code_prefix(&self) -> &'static str {
            "C-L"
        }

        fn message(&self) -> String {
            use LexerErrorKind::*;
            match *self {
                Utf8Decode(ref error) => format!(
                    "{}. Only UTF-8 text is supported.",
                    error
                ),
                Io(ref error) => format!(
                    "An IO error occured. {}",
                    error
                ),
                MissingCorrespondingIf(ref end_token) => format!(
                    "{} does not have a corresponding #if, #ifdef, #ifndef, or #elif.",
                    end_token
                ),
                MissingCorrespondingEndIf(ref start_token) => format!(
                    "{} does not have a corresponding #elif, #else, or #endif.",
                    start_token
                ),
                UnendedComment => {
                    "The multiline comment was not properly ended before the end of the file."
                        .to_owned()
                },
                UnendedInclude(is_sys) => format!(
                    "Include was not properly ended with a {} before the end of the line.",
                    if is_sys { '>' } else { '"' }
                ),
                UnendedString(is_char) => format!(
                    "{} was not ended properly with a {} before the end of the line.",
                    if is_char { "Character" } else { "String" },
                    if is_char { '\'' } else { '"' }
                ),
            }
        }
    }
}

impl From<std::io::Error> for LexerErrorKind {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.into())
    }
}

impl From<Utf8DecodeError> for LexerErrorKind {
    fn from(error: Utf8DecodeError) -> Self {
        Self::Utf8Decode(error)
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    error::Severity,
    sync::Arc,
    util::{
        enum_with_properties,
        PtrEquality,
        SourceLocation,
        Utf8DecodeError,
    },
};

#[derive(Clone, Debug)]
pub struct LexerError {
    location: SourceLocation,
    kind: LexerErrorKind,
}
impl LexerError {
    pub fn new(location: SourceLocation, kind: LexerErrorKind) -> Self {
        LexerError { location, kind }
    }

    pub fn location(&self) -> &SourceLocation {
        &self.location
    }

    pub fn severity(&self) -> Severity {
        self.kind.severity()
    }

    pub fn code(&self) -> &'static str {
        self.kind.code()
    }
}

enum_with_properties! {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum LexerErrorKind {
        // == Fatals
        #[values(Fatal, "LF400")]
        Utf8Decode(Utf8DecodeError),
        #[values(Fatal, "LF410")]
        Io(PtrEquality<Arc<std::io::Error>>),
        // == Errors
        #[values(Error, "LE300")]
        MissingCorrespondingIf,
        #[values(Error, "LE301")]
        MissingCorrespondingEndIf,
        #[values(Error, "LE310")]
        UnendedComment,
        #[values(Error, "LE311")]
        UnendedInclude,
        #[values(Error, "LE312")]
        UnendedString,
    }

    impl LexerErrorKind {
        #[property]
        pub fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        pub fn code(&self) -> &'static str {}
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

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    sync::Arc,
    util::{
        enum_with_properties,
        PtrEquality,
        Severity,
        SourceLocation,
        Utf8DecodeError,
    },
};

enum_with_properties! {
    pub fn severity(&self) -> Severity {
        use Severity::*;
    }
    pub fn code(&self) -> &'static str {}

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum LexerError {
        // == Fatals
        #[values(Fatal, "LF400")]
        Utf8Decode(Utf8DecodeError),
        #[values(Fatal, "LF410")]
        Io(PtrEquality<Arc<std::io::Error>>),
        // == Errors
        #[values(Error, "LE300")]
        MissingCorrespondingIf(SourceLocation),
        #[values(Error, "LE301")]
        MissingCorrespondingEndIf(SourceLocation),
        #[values(Error, "LE310")]
        UnendedComment(SourceLocation),
        #[values(Error, "LE311")]
        UnendedInclude(SourceLocation),
        #[values(Error, "LE312")]
        UnendedString(SourceLocation),
    }
}

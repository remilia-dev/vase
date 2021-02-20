// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    sync::Arc,
    util::{
        enum_with_properties,
        CachedString,
        PtrEquality,
        Severity,
        Utf8DecodeError,
    },
};

enum_with_properties! {
    pub fn severity(&self) -> Severity {
        use Severity::*;
    }
    pub fn code(&self) -> &'static str {}

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum CLexerError {
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
}

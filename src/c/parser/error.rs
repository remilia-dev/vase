// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::*,
        TravelerError,
        TravelerErrorKind,
        TravelerState,
    },
    error::{
        CodedError,
        Severity,
    },
    util::enum_with_properties,
};

#[derive(Clone, Debug)]
pub struct ParseError {
    pub state: TravelerState,
    pub kind: ParseErrorKind,
}

impl CodedError for ParseError {
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

impl From<TravelerError> for ParseError {
    fn from(error: TravelerError) -> Self {
        ParseError {
            state: error.state,
            kind: ParseErrorKind::Travel(error.kind),
        }
    }
}

enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum ParseErrorKind {
        // == Others
        #[values(v0.severity(), v0.code_number())]
        Number(NumberError),
        #[values(v0.severity(), v0.code_number())]
        Travel(TravelerErrorKind),
        // == Internals
        #[values(Internal, 900)]
        Unimplemented(&'static str),
        #[values(Internal, 901)]
        Unreachable(&'static str),
    }

    impl CodedError for ParseErrorKind {
        #[property]
        fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        fn code_number(&self) -> u32 {}

        fn code_prefix(&self) -> &'static str {
            match *self {
                Self::Travel(ref error) => error.code_prefix(),
                Self::Number(ref error) => error.code_prefix(),
                _ => "C-P",
            }
        }

        fn message(&self) -> String {
            use ParseErrorKind::*;
            match *self {
                Number(ref error) => error.message(),
                Travel(ref error) => error.message(),
                Unimplemented(thing) => format!(
                    "{} is currently unimplemented.",
                    thing
                ),
                Unreachable(thing) => format!(
                    "Unreachable condition: {}. This is an internal error.",
                    thing
                ),
            }
        }
    }
}

impl From<NumberError> for ParseErrorKind {
    fn from(error: NumberError) -> Self {
        ParseErrorKind::Number(error)
    }
}

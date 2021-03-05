// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

// Only two types should be publicly used (the traveler and it's save-state).
pub use error::{
    TravelerError,
    TravelerErrorKind,
};
pub use implementation::Traveler;
pub use state::TravelerState;

// These uses are to allow the various files in this module to interact.
pub(self) use self::frame::*;
pub(self) use self::frame_stack::*;
pub(self) use self::if_evaluator::IfEvaluator;
pub(self) use self::if_parser::IfParser;
pub(self) use self::macro_kind::*;

mod error;
mod frame;
mod frame_stack;
mod if_evaluator;
mod if_parser;
mod implementation;
mod macro_kind;
mod state;

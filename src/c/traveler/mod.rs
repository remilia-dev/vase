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
// The self:: prefix is to prevent rustfmt from mixing these with the public imports above.
pub(self) use self::{
    frame::*,
    frame_stack::*,
    if_evaluator::IfEvaluator,
    if_parser::IfParser,
    macro_kind::*,
};

mod error;
mod frame;
mod frame_stack;
mod if_evaluator;
mod if_parser;
mod implementation;
mod macro_kind;
mod state;

pub type TravelIndex = crate::math::NonMaxU32;
pub type TravelRange = std::ops::Range<TravelIndex>;

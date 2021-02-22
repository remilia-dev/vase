// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

// Only two types should be publicly used (the traveler and it's save-state).
pub use frame_stack::TravelerState;
pub use implementation::Traveler;

// These uses are to allow the various files in this module to interact.
pub(self) use self::frame::*;
pub(self) use self::frame_stack::*;
pub(self) use self::macro_kind::*;

mod frame;
mod frame_stack;
mod implementation;
mod macro_kind;

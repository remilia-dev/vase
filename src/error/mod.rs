// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use coded::CodedError;
pub use severity::Severity;
pub use unwind::{
    MayUnwind,
    Unwind,
};

mod coded;
mod severity;
mod unwind;

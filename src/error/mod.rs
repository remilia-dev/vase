// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use coded::CodedError;
pub use receiver::ErrorReceiver;
pub use severity::Severity;
pub use unwind::{
    MayUnwind,
    Unwind,
};

mod coded;
mod receiver;
mod severity;
mod unwind;

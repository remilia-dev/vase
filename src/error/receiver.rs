// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::cell::RefCell;

use crate::{
    error::{
        CodedError,
        MayUnwind,
        Unwind,
    },
    sync::Arc,
};

pub trait ErrorReceiver<E: CodedError> {
    /// Reports that an error has occured.
    ///
    /// Returns true if the error should be considered fatal.
    fn report_error(&mut self, error: E) -> bool;
    /// Much like report_error, this reports that an error has occured.
    ///
    /// If the error is considered fatal, a fatal unwinding will be returned.
    /// Otherwise, execution will continue on.
    fn report(&mut self, error: E) -> MayUnwind<()> {
        let mut fatal = error.severity().is_fatal();
        fatal |= self.report_error(error);
        if fatal { Err(Unwind::Fatal) } else { Ok(()) }
    }
}

impl<E, F> ErrorReceiver<E> for F
where
    E: CodedError,
    F: FnMut(E) -> bool,
{
    fn report_error(&mut self, error: E) -> bool {
        self(error)
    }
}

impl<E, T> ErrorReceiver<E> for Arc<RefCell<T>>
where
    E: CodedError,
    T: ErrorReceiver<E>,
{
    fn report_error(&mut self, error: E) -> bool {
        self.borrow_mut().report_error(error)
    }
}

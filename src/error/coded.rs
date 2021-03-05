// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::error::Severity;
/// An error type with a severity and code.
pub trait CodedError {
    /// The severity of the error.
    fn severity(&self) -> Severity;
    /// The code number of this error.
    /// # Code Numbers by Severity
    /// The code number should fit the error's severity:
    /// * 100-199 for deprecations
    /// * 200-499 for warnings
    /// * 500-799 for normal errors
    /// * 800-899 for fatal errors
    /// * 900-999 for internal errors
    /// # Not Unique
    /// This value is *not* unique in the entire program.
    /// See [code](CodedError::code) for a unique value.
    fn code_number(&self) -> u32;
    /// The code prefix for this error. This will general follow the
    /// format `X-Y` where X is the language the error occured in and Y
    /// is the part of the compiler the error occured in (like L for Lexer).
    ///
    /// For example the `C-L` prefix indicates an error in the lexing
    /// of C code.
    fn code_prefix(&self) -> &'static str;
    /// The combination of [code_prefix](CodedError::code_prefix)
    /// and [code_number](CodedError::code_number).
    fn code(&self) -> String {
        format!("{}{}", self.code_prefix(), self.code_number())
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use codespan_reporting::diagnostic as crd;

/// An enum indicating the severity of an error.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    /// A fatal error that signals one of two things:
    /// * an internal error that *should* not occur.
    /// * something that has not been implemented but is required.
    Internal,
    /// An error that stops further compilation as it's practically unrecoverable.
    Fatal,
    /// An error that *may* be able to be ignored to proceed to the next step.
    /// If too many errors occur, compilation should be stopped and only report the first few errors.
    Error,
    /// A warning about a piece of code. Warnings can be enabled/disabled.
    Warning,
    /// A warning to signal that a piece of code/behavior is deprecated.
    Deprecation,
}

impl Severity {
    /// Returns whether this severity is considered fatal or not.
    /// This is currently the Internal and Fatal severities.
    pub fn is_fatal(self) -> bool {
        matches!(self, Self::Internal | Self::Fatal)
    }
}

impl From<Severity> for crd::Severity {
    /// Coverts this severity to the one used by Codespan Reporting.
    /// # Note
    /// Some severities return the same codespan severities.
    fn from(severity: Severity) -> crd::Severity {
        match severity {
            Severity::Internal => crd::Severity::Bug,
            Severity::Fatal => crd::Severity::Error,
            Severity::Error => crd::Severity::Error,
            Severity::Warning => crd::Severity::Warning,
            Severity::Deprecation => crd::Severity::Warning,
        }
    }
}

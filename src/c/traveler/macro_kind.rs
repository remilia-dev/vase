// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::c::{
    traveler::Frame,
    CToken,
    FileId,
};

/// A enum representing the different types of macros.
#[derive(Clone, Debug)]
pub(super) enum MacroKind {
    /// An object macro that contains no tokens.
    Empty,
    /// An object macro that contains a single token.
    SingleToken { token: CToken },
    /// An object macro that contains at least two tokens.
    ObjectMacro {
        /// The file id the macro was defined in.
        file_id: FileId,
        /// The index of the first token of the macro.
        index: usize,
        /// The index at which this macro should be considered complete.
        ///
        /// This should be the index of the [PreEnd](crate::c::CTokenKind::PreEnd).
        end: usize,
    },
    /// A function macro.
    FuncMacro {
        /// The file id the macro was defined in.
        file_id: FileId,
        /// The index of the first token of the macro.
        index: usize,
        /// The index at which this macro should be considered complete.
        ///
        /// This should be the index of the [PreEnd](crate::c::CTokenKind::PreEnd).
        end: usize,
        /// A list containing each parameter's unique id in order.
        param_ids: Vec<usize>,
        /// A value representing the potential unique id of the var-arg.
        /// * If this function macro doesn't have a var-arg, this will be None.
        /// * If this function macro doesn't define a name, "__VA_ARGS__" will be used.
        /// * If a name was provided, it will be the unique id of that name.
        var_arg: Option<usize>,
    },
}

/// An enum that represents the type of macro that [FrameStack](super::FrameStack)
/// should handle.
pub(super) enum MacroHandle {
    /// An empty macro that should be handled. The FrameStack should move past the current token.
    Empty,
    /// A macro that can be handled by pushing a pre-calculated frame.
    ///
    /// The macro should be a single-token macro, an object-macro, or a function-macro's argument.
    Simple(Frame),
    /// A function macro that must be handled.
    ///
    /// This is handled separately since it requires reading the parameter tokens.
    FuncMacro { macro_id: usize, param_count: usize },
}

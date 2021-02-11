// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::HashMap;

use crate::c::{
    CToken,
    FileId,
};

#[derive(Clone, Debug)]
pub(super) enum Frame {
    /// A frame that represents tokens in a file.
    ///
    /// This frame could be for the root file or includes.
    File {
        /// The id of the file to get tokens from.
        file_id: FileId,
        /// The current index of the token we are at.
        index: usize,
        /// The index this frame would be complete at.
        ///
        /// For include frames, this will *exclude* the [Eof](crate::c::CTokenKind::Eof) token.
        end: usize,
    },
    /// A frame that represents a single token.
    ///
    /// This token could be the result of a joined token or an object macro with only one token.
    SingleToken {
        /// The token this frame represents.
        token: CToken,
        /// The unique id of the object macro this is from.
        ///
        /// If this value is usize::MAX, then this is actually from a token-join operator.
        macro_id: usize,
    },
    /// A frame that represents an object macro with 2 tokens or more.
    /// # 1-Token and 0-Token Object Macros
    /// * 1-Token object macros are handled by SingleToken.
    /// * 0-Token object macros are handled separately.
    ObjectMacro {
        /// The id of the file to get tokens from.
        file_id: FileId,
        /// The current index of token we are at.
        index: usize,
        /// The index this frame would be complete at.
        ///
        /// This will always exclude the [PreEnd](crate::c::CTokenKind::PreEnd) token.
        end: usize,
        /// The unique id of the object macro this is from.
        macro_id: usize,
    },
    /// A frame that represents a function macro.
    FuncMacro {
        /// The id of the file to get tokens from.
        ///
        /// This is the file that the macro was defined in.
        file_id: FileId,
        /// The current index of token we are at.
        index: usize,
        /// The index this frame would be complete at.
        ///
        /// This will always exclude the [PreEnd](crate::c::CTokenKind::PreEnd) token.
        end: usize,
        /// The unique id of the function macro this is from.
        macro_id: usize,
        /// A map from a parameter's unique id to the tokens that were given for that parameter.
        params: HashMap<usize, Vec<CToken>>,
    },
    /// A frame that represents a function macro's parameter being used.
    ///
    /// A FuncArg frame should *always* be preceded by a FuncMacro frame.
    /// This is because the parameter's tokens are read from that frame.
    FuncArg {
        /// The current index of the token we are at.
        index: usize,
        /// The index this frame would be complete at.
        ///
        /// It should be the length of the parameter's token array.
        end: usize,
        /// The unique id of the parameter to get tokens from.
        param_id: usize,
    },
}

impl Frame {
    /// Increments the index of this frame.
    ///
    /// Returns if the frame has ran out of values.
    pub fn increment_index(&mut self) -> bool {
        use Frame::*;
        match self {
            // Single tokens have no index to increment.
            SingleToken { .. } => false,
            File { index, end, .. }
            | ObjectMacro { index, end, .. }
            | FuncMacro { index, end, .. }
            | FuncArg { index, end, .. } => {
                *index = index.wrapping_add(1);
                index < end
            },
        }
    }
    /// Gets a token of a parameter's token list by it's index.
    /// # Panics
    /// If this frame is not a FuncMacro frame.
    pub fn get_param_token(&self, param_id: usize, index: usize) -> &CToken {
        if let Frame::FuncMacro { params, .. } = self {
            &params.get(&param_id).unwrap()[index]
        } else {
            panic!("get_param should only be called on a FuncMacro frame!")
        }
    }
}

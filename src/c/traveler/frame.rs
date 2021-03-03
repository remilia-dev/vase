// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::HashMap;

use crate::{
    c::{
        traveler::MacroHandle,
        Token,
    },
    sync::Arc,
    util::FileId,
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
        token: Token,
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
        /// The current index of the token we are at.
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
        /// The tokens that result from combining the function macro and its arguments.
        ///
        /// This is list can be very massive, it is contained in an Arc for when the traveler
        /// state is saved.
        tokens: Arc<Vec<Token>>,
        /// The id of the file to get tokens from.
        ///
        /// This is the file that the macro was defined in.
        //file_id: FileId,
        /// The current index of token we are at.
        index: usize,
        /// The unique id of the function macro this is from.
        macro_id: usize,
    },
    /// A frame that is used to collect the tokens for a function macro.
    TokenCollector {
        /// The id of the file to get tokens from.
        file_id: FileId,
        /// The current index of the token we are at.
        index: usize,
        /// The index this frame would be complete at.
        ///
        /// This will always *include* the [PreEnd](crate::c::CTokenKind::PreEnd) token.
        end: usize,
        /// A map from a unique id to the tokens the parameter makes up.
        params: HashMap<usize, Vec<Token>>,
    },
    /// A frame that represents a token collector's parameter.
    ///
    /// This frame should *always* be preceded by a TokenCollector frame.
    TokenCollectorParameter {
        /// The current index of the token we are at.
        index: usize,
        /// The index this frame would be complete at.
        end: usize,
        /// The id of the parameter to get tokens from.
        param_id: usize,
    },
}

impl Frame {
    pub fn get_file_id(&self) -> FileId {
        use Frame::*;
        match *self {
            File { file_id, .. } | ObjectMacro { file_id, .. } => file_id,
            FuncMacro { ref tokens, index, .. } => tokens[index].location().file_id(),
            SingleToken { ref token, .. } => token.location().file_id(),
            TokenCollector { .. } | TokenCollectorParameter { .. } => panic!(
                "Can't get the file id on token collector frames! No analysis should be performed within these frames."
            ),
        }
    }
    /// Increments the index of this frame.
    ///
    /// Returns true if the frame has more tokens.
    pub fn increment_index(&mut self) -> bool {
        use Frame::*;
        match *self {
            // Single tokens have no index to increment.
            SingleToken { .. } => false,
            File { ref mut index, end, .. }
            | ObjectMacro { ref mut index, end, .. }
            | TokenCollector { ref mut index, end, .. }
            | TokenCollectorParameter { ref mut index, end, .. } => {
                *index = index.wrapping_add(1);
                *index < end
            },
            FuncMacro { ref mut index, ref tokens, .. } => {
                *index = index.wrapping_add(1);
                *index < tokens.len()
            },
        }
    }

    pub fn has_parameter(&self, param_id: usize) -> Option<MacroHandle> {
        match *self {
            Frame::TokenCollector { ref params, .. } => {
                let param_vec = params.get(&param_id)?;
                Some(if param_vec.is_empty() {
                    MacroHandle::Empty
                } else {
                    MacroHandle::Simple(Frame::TokenCollectorParameter {
                        param_id,
                        index: 0,
                        end: param_vec.len(),
                    })
                })
            },
            _ => None,
        }
    }

    pub fn stringify(&self, param_id: usize) -> Option<String> {
        use std::fmt::Write;
        match *self {
            Frame::TokenCollector { ref params, .. } => {
                let mut buffer = String::new();
                let params = params.get(&param_id)?;
                for (i, param) in params.iter().enumerate() {
                    if i != 0 && param.whitespace_before() {
                        buffer.push(' ');
                    }
                    write!(buffer, "{}", param.kind())
                        .expect("Formating a token *should* never fail.");
                }
                Some(buffer)
            },
            _ => None,
        }
    }
}

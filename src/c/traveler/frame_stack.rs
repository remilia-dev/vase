// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::{
    HashMap,
    VecDeque,
};

use crate::{
    c::{
        traveler::{
            Frame,
            MacroHandle,
            MacroKind,
            TravelerError,
            TravelerState,
        },
        CompileEnv,
        FileTokens,
        Token,
        TokenKind,
        TokenKind::*,
    },
    error::{
        ErrorReceiver,
        MayUnwind,
    },
    sync::Arc,
    util::{
        CachedString,
        FileId,
    },
};

type Error = crate::c::TravelerErrorKind;
type Receiver<'a> = &'a mut dyn ErrorReceiver<TravelerError>;

/// A manager struct for where [Traveler](super::Traveler) is in a token stack.
///
/// This includes reading tokens from macros and includes. It is important to note
/// that FrameStack *never* handles preprocessor instructions (CTraveler does).
pub(super) struct FrameStack<'a> {
    env: &'a CompileEnv,
    /// A map from file ids to the token stacks. Token stacks are loaded into here as needed.
    file_refs: HashMap<FileId, Arc<FileTokens>>,
    /// A vec-deque of frames. The frame that is currently being worked on will always be at index 0.
    frames: VecDeque<Frame>,
    /// A list of all the files that have been read so far during travel.
    dependencies: Vec<FileId>,
    /// A map from a macro's unique id to the kind of macro it is.
    ///
    /// A macro's unique id is the uniq_id() of its identifier.
    macros: HashMap<usize, MacroKind>,
    /// Whether CTraveler should skip-ahead on PreElseIf/PreElse tokens.
    ///
    /// This is set to true every time the stack is moved. The only way it is false
    /// is if skip_to is used.
    should_chain_skip: bool,
    /// If we were to restart, how many calls of [Traveler.move_forward] would
    /// be needed to get to where we are.
    ///
    /// This value is stored in the frame stack since it is saved.
    pub(super) index: u32,
}

impl<'a> FrameStack<'a> {
    /// Creates a new frame stack from the given compile environment.
    pub fn new(env: &'a CompileEnv) -> Self {
        // OPTIMIZATION: A different hasher may be more performant
        FrameStack {
            env,
            file_refs: HashMap::default(),
            frames: VecDeque::default(),
            dependencies: Vec::new(),
            macros: HashMap::default(),
            should_chain_skip: true,
            index: 0,
        }
    }
    /// Sets up the frame stack up to start processing the given token stack.
    ///
    /// This removes all previous macros/frames.
    pub fn load_start(&mut self, tokens: Arc<FileTokens>) {
        self.frames.clear();
        self.macros.clear();
        self.dependencies.clear();
        self.should_chain_skip = true;
        self.index = 0;

        self.frames.push_front(Frame::File {
            file_id: tokens.file_id(),
            end: tokens.len(),
            index: usize::MAX,
        });
        self.file_refs.insert(tokens.file_id(), tokens);
    }
    /// Whether CTraveler should skip-ahead on PreElseIf/PreElse tokens.
    ///
    /// This is set to true every time the stack is moved. The only way it is false
    /// is if skip_to is used.
    pub fn should_chain_skip(&self) -> bool {
        self.should_chain_skip
    }
    /// Returns a saved state that can be used later to return to the current location.
    pub fn save_state(&self) -> TravelerState {
        // OPTIMIZATION: self.macros should use a COW structure to avoid unnecessary clones
        TravelerState {
            frames: self.frames.clone(),
            macros: self.macros.clone(),
            dependencies: self.dependencies.clone(),
            should_chain_skip: self.should_chain_skip,
            index: self.index,
        }
    }
    /// Loads the given saved state.
    /// # Panics
    /// Panics can occur if this state is from a different frame stack or this frame stack has
    /// been reused since this state. These panics won't occur on this function call, they'll
    /// occur later in the usage of the stack.
    pub fn load_state(&mut self, state: TravelerState) {
        self.frames = state.frames;
        self.macros = state.macros;
        self.dependencies = state.dependencies;
        self.should_chain_skip = state.should_chain_skip;
        self.index = state.index;
    }
    /// Returns a reference to the current token the frame stack is at.
    pub fn head(&self) -> &Token {
        match self.frames[0] {
            Frame::File { file_id, index, .. }
            | Frame::ObjectMacro { file_id, index, .. }
            | Frame::TokenCollector { file_id, index, .. } => &self.file_refs[&file_id][index],
            Frame::SingleToken { ref token, .. } => token,
            Frame::FuncMacro { index, ref tokens, .. } => &tokens[index],
            Frame::TokenCollectorParameter { index, param_id, .. } => {
                if let Frame::TokenCollector { ref params, .. } = self.frames[1] {
                    &params[&param_id][index]
                } else {
                    panic!(
                        "TokenCollectorParameter frame should have been preceded by TokenCollector frame."
                    );
                }
            },
        }
    }
    /// Attempts to get a preview of the next token.
    ///
    /// This can fail or return a mildly incorrect result. This can occur when:
    /// * The next token is outside the current macro and `exit_macros` is false.
    /// * The next token is a function parameter
    ///
    /// Most of the time when the next token is outside the current macro and `exit_macros` is false.
    pub fn preview_next_kind(&self, exit_macros: bool) -> Option<&TokenKind> {
        for i in 0..self.frames.len() {
            match self.frames[i] {
                Frame::File { file_id, index, end, .. } => {
                    let file = &self.file_refs[&file_id];
                    if index + 1 < end {
                        return Some(file[index + 1].kind());
                    }
                },
                Frame::SingleToken { .. } => {
                    if !exit_macros {
                        return None;
                    }
                },
                Frame::ObjectMacro { file_id, index, end, .. } => {
                    if index + 1 < end {
                        return Some(self.file_refs[&file_id][index + 1].kind());
                    } else if !exit_macros {
                        return None;
                    }
                },
                Frame::FuncMacro { index, ref tokens, .. } => {
                    if index + 1 > tokens.len() {
                        if exit_macros {
                            continue;
                        } else {
                            return None;
                        }
                    }

                    return Some(tokens[index + 1].kind());
                },
                Frame::TokenCollector { .. } => {
                    // We don't need look-ahead in TokenCollector frames as we're collecting the tokens here.
                    // The only special action that occurs in token collector frames is handling
                    // parameter substitution.
                    return None;
                },
                Frame::TokenCollectorParameter { param_id, index, end, .. } => {
                    return if index + 1 >= end {
                        None
                    } else if let Frame::TokenCollector { ref params, .. } = self.frames[i + 1] {
                        Some(params[&param_id][index + 1].kind())
                    } else {
                        panic!(
                            "TokenCollectorParameter frame should have been preceded by TokenCollector frame."
                        )
                    };
                },
            }
        }

        None
    }
    /// Moves the stack to the next token.
    ///
    /// This will remove any frames that we have reached the end of.
    pub fn move_forward(&mut self) -> &Token {
        self.should_chain_skip = true;
        while !self.frames[0].increment_index() {
            self.frames.pop_front();
        }
        self.head()
    }

    pub fn get_current_file(&self) -> &FileTokens {
        &self.file_refs[&self.frames[0].get_file_id()]
    }
    /// Gets the file id of the given include string.
    ///
    /// This will only examine the top file frame of the stack.
    pub fn get_include_ref(&mut self, inc_str: &CachedString) -> Option<FileId> {
        for frame in &self.frames {
            if let Frame::File { file_id, .. } = *frame {
                return self.file_refs[&file_id].get_file_ref(inc_str);
            }
        }

        None
    }
    /// Gets the file id and index of the current frame.
    /// # Panics
    /// Panics if the frame stack is currently not on a file frame.
    pub fn get_file_index(&self) -> (FileId, usize) {
        match self.frames[0] {
            Frame::File { file_id, index, .. } => (file_id, index),
            _ => panic!("Can only get the file index if the last frame was a file frame."),
        }
    }
    /// Returns true if a token joiner is the 'next' token.
    pub fn is_token_joiner_next(&self) -> bool {
        let frame = match self.frames[0] {
            Frame::SingleToken { .. } => &self.frames[1],
            ref frame => frame,
        };
        match *frame {
            Frame::ObjectMacro { file_id, index, end, .. }
            | Frame::TokenCollector { file_id, index, end, .. } => {
                // We want to exclude searching for a token joiner at the end of the macro.
                if index + 1 < end - 1 {
                    let next_token = self.file_refs[&file_id][index + 1].kind();
                    matches!(*next_token, HashHash { .. })
                } else {
                    false
                }
            },
            Frame::FuncMacro { index, ref tokens, .. } => {
                // We want to exclude searching for a token joiner at the end of the macro.
                if tokens.is_empty() {
                    false
                } else if index + 1 < tokens.len() - 1 {
                    let next_token = tokens[index + 1].kind();
                    matches!(*next_token, HashHash { .. })
                } else {
                    false
                }
            },
            _ => false,
        }
    }
    /// Attempts to push a file frame to include another token stack (by its file id).
    ///
    /// This will return Err only if no token stack by that file id could be loaded.
    pub fn push_include(&mut self, file_id: FileId) -> Result<(), ()> {
        self.dependencies.push(file_id);
        let (file_id, length) = match self.file_refs.get(&file_id) {
            Some(file) => (file_id, file.len()),
            None => match self.env.file_id_to_tokens.get_arc(file_id) {
                Some(tokens) => {
                    let length = tokens.len();
                    self.file_refs.insert(file_id, tokens);
                    (file_id, length)
                },
                None => return Err(()),
            },
        };

        self.frames.push_front(Frame::File {
            file_id,
            index: 0,
            // The -1 is to exclude the EOF token.
            end: length - 1,
        });
        Ok(())
    }
    /// Pushes a single-token frame onto the stack.
    ///
    /// This method should only be used for token-joiner and stringification operations.
    pub fn push_token(&mut self, token: Token) {
        let frame = Frame::SingleToken { macro_id: usize::MAX, token };
        self.frames.push_front(frame);
    }
    /// Skips the file frame to the given link. You can also set whether the skip should
    /// chain (keep jumping till past any PreElseIf/PreElse tokens).
    /// # Panics
    /// Panics if the current frame is not a file frame. Skipping should not occur inside
    /// any macros.
    pub fn skip_to(&mut self, link: usize, should_chain_skip: bool) {
        self.should_chain_skip = should_chain_skip;
        match self.frames[0] {
            Frame::File { ref mut index, .. } => *index = link as usize,
            _ => panic!("Can only skip to link when the last frame is an file frame."),
        }
    }
}

// Macro Utilities
impl<'a> FrameStack<'a> {
    /// Returns whether the given macro unique-id has been defined.
    pub fn has_macro(&self, macro_id: usize) -> bool {
        self.macros.contains_key(&macro_id)
    }
    /// Sets that a unique id represents the given macro.
    /// This does not check if any previous macros were the same.
    pub fn add_macro(&mut self, macro_id: usize, mcr: MacroKind) {
        self.macros.insert(macro_id, mcr);
    }
    /// Removes the given macro unique-id as being defined.
    pub fn remove_macro(&mut self, macro_id: usize) {
        self.macros.remove(&macro_id);
    }
    /// Checks if the given unique id should be handled as a macro.
    /// This will return None should any of the following occur:
    /// * The unique id is not the unique id of a macro.
    /// * The macro is already in-use in the frame stack.
    ///
    /// Should some value be returned, the value contains the strategy [FrameStack::handle_macro] should use.
    pub fn should_handle_macro(&self, macro_id: usize) -> Option<MacroHandle> {
        let mcr = self.macros.get(&macro_id)?;

        if self.in_macro(macro_id) {
            return None;
        }

        match *mcr {
            MacroKind::Empty => Some(MacroHandle::Empty),
            MacroKind::SingleToken { ref token } => {
                let frame = Frame::SingleToken { token: token.clone(), macro_id };
                Some(MacroHandle::Simple(frame))
            },
            MacroKind::ObjectMacro { index, file_id, end } => {
                let frame = Frame::ObjectMacro { file_id, index, end, macro_id };
                Some(MacroHandle::Simple(frame))
            },
            MacroKind::FuncMacro { ref param_ids, .. } => {
                let param_count = param_ids.len();
                if let Some(&TokenKind::LParen) = self.preview_next_kind(true) {
                    Some(MacroHandle::FuncMacro { macro_id, param_count })
                } else {
                    None
                }
            },
        }
    }

    pub fn handle_macro(&mut self, handle: MacroHandle, errors: Receiver) -> MayUnwind<()> {
        match handle {
            MacroHandle::Empty => {
                // Move past the empty token.
                self.move_forward();
            },
            MacroHandle::Simple(frame) => {
                // Add the frame that was already calculated.
                self.frames.push_front(frame)
            },
            MacroHandle::FuncMacro { macro_id, param_count } => {
                self.handle_function_macro(macro_id, param_count, errors)?;
            },
        }
        Ok(())
    }

    fn handle_function_macro(
        &mut self,
        macro_id: usize,
        param_count: usize,
        errors: Receiver,
    ) -> MayUnwind<()> {
        // Pass the ID of the macro
        self.move_forward();

        let mut param_tokens = self.collect_func_macro_invocation(param_count, errors)?;

        if let MacroKind::FuncMacro {
            file_id,
            index,
            end,
            ref param_ids,
            var_arg,
        } = self.macros[&macro_id]
        {
            let id_count = param_ids.len();
            let param_count = param_tokens.len();

            let var_arg_tokens = if var_arg.is_some() && param_count > id_count {
                param_tokens.pop()
            } else {
                None
            };

            let mut param_map: HashMap<usize, Vec<Token>> =
                param_ids.iter().copied().zip(param_tokens).collect();
            if param_count < id_count {
                self.report_error(
                    Error::FuncInvokeMissingArgs(id_count - param_count),
                    errors,
                )?;
                for id in &param_ids[param_count..id_count] {
                    param_map.insert(*id, Vec::new());
                }
            }

            match (var_arg, var_arg_tokens) {
                (Some(id), Some(tokens)) => {
                    param_map.insert(id, tokens);
                },
                (Some(id), None) => {
                    param_map.insert(id, Vec::new());
                },
                (None, Some(tokens)) => {
                    self.report_error(Error::FuncInvokeExcessParameters(tokens), errors)?;
                },
                (None, None) => {},
            }

            self.create_func_macro_frame(file_id, index, end, macro_id, param_map, errors)
        } else {
            panic!("Can't handle a function macro on a non-function macro.");
        }
    }

    fn create_func_macro_frame(
        &mut self,
        file_id: FileId,
        index: usize,
        end: usize,
        macro_id: usize,
        params: HashMap<usize, Vec<Token>>,
        errors: Receiver,
    ) -> MayUnwind<()> {
        // By assuming each parameter will show up at least once, we get a good initial capacity estimation.
        let sum_parameter_lengths = params.iter().fold(0, |accum, value| accum + value.1.len());

        // This frame is to read the tokens in a function macro.
        self.frames.push_front(Frame::TokenCollector {
            file_id,
            index,
            // We want to include the PreEnd token to signal to
            end: end + 1,
            params,
        });

        let function_frame = self.frames.len();

        let mut tokens = Vec::with_capacity(sum_parameter_lengths);
        loop {
            let head = self.head();
            match *head.kind() {
                PreEnd if self.frames.len() == function_frame => {
                    break;
                },
                Hash { .. } if self.frames.len() == function_frame => {
                    let loc = head.loc();
                    let define = match self.move_forward().kind() {
                        token if token.is_definable() => token.get_definable_id(),
                        _ => {
                            let error = Error::StringifyExpectsId(self.head().clone());
                            self.report_error(error, errors)?;
                            continue;
                        },
                    };

                    let str_data = if let Some(string) = self.frames[0].stringify(define) {
                        Arc::new(string.into_boxed_str())
                    } else {
                        let id_token = self.head().clone();
                        let id = match *id_token.kind() {
                            Identifier(ref id) => id.clone(),
                            _ => self.env.cache().get_or_cache(id_token.kind().text()),
                        };
                        let error = Error::StringifyNonParameter(id_token);
                        self.report_error(error, errors)?;
                        Arc::new(Box::from(id.string()))
                    };
                    tokens.push(Token::new(loc, true, String {
                        encoding: crate::c::StringEnc::Default,
                        has_escapes: false,
                        is_char: false,
                        str_data,
                    }));
                },
                ref def if def.is_definable() && self.frames.len() == function_frame => {
                    let param_id = def.get_definable_id();
                    match self.frames[0].has_parameter(param_id) {
                        Some(handle) if handle.is_empty() => {
                            let last_kind = tokens.last().map(|token| token.kind());
                            if let Some(&HashHash { .. }) = last_kind {
                                tokens.pop();
                            } else if self.is_token_joiner_next() {
                                self.move_forward();
                            }
                            self.handle_macro(handle, errors)?;
                            continue;
                        },
                        Some(handle) => {
                            self.handle_macro(handle, errors)?;
                            continue;
                        },
                        None => tokens.push(head.clone()),
                    }
                },
                ref def if def.is_definable() => {
                    let macro_id = def.get_definable_id();
                    if let Some(handle) = self.should_handle_macro(macro_id) {
                        self.handle_macro(handle, errors)?;
                        continue;
                    } else {
                        tokens.push(head.clone());
                    }
                },
                _ => tokens.push(head.clone()),
            }

            self.move_forward();
        }

        self.frames.pop_front();
        if tokens.is_empty() {
            self.move_forward();
        } else {
            self.frames.push_front(Frame::FuncMacro {
                macro_id,
                index: 0,
                tokens: Arc::new(tokens),
            });
        }
        Ok(())
    }

    fn collect_func_macro_invocation(
        &mut self,
        param_count: usize,
        errors: Receiver,
    ) -> MayUnwind<Vec<Vec<Token>>> {
        let mut param_tokens = vec![Vec::new()];
        let mut paren_layers = 0usize;
        let mut in_preprocessor = false;
        loop {
            let head = self.move_forward();
            match *head.kind() {
                LParen => paren_layers += 1,
                RParen => {
                    if paren_layers == 0 && !in_preprocessor {
                        break;
                    } else {
                        paren_layers -= 1;
                    }
                },
                Comma if paren_layers == 0 && !in_preprocessor => {
                    if param_tokens.len() <= param_count {
                        param_tokens.push(Vec::new());
                        continue;
                    }
                },
                _ if head.kind().is_preprocessor() => {
                    param_tokens.last_mut().unwrap().push(head.clone());
                    let error = Error::FuncInvokePreprocessorInArgs(head.clone());
                    self.report_error(error, errors)?;
                    in_preprocessor = true;
                    continue;
                },
                PreEnd => {
                    if in_preprocessor {
                        in_preprocessor = false;
                        param_tokens.last_mut().unwrap().push(head.clone());
                    } else {
                        self.report_error(Error::InnerFuncInvokeUnfinished, errors)?;
                        break;
                    }
                },
                Eof => {
                    self.report_error(Error::InnerFuncInvokeUnfinished, errors)?;
                    break;
                },
                _ => {},
            }

            param_tokens.last_mut().unwrap().push(head.clone());
        }
        Ok(param_tokens)
    }

    fn report_error(&self, kind: Error, errors: Receiver) -> MayUnwind<()> {
        use crate::error::CodedError;
        let mut fatal = kind.severity().is_fatal();

        fatal |= errors.report_error(TravelerError { state: self.save_state(), kind });

        if fatal {
            Err(crate::error::Unwind::Fatal)
        } else {
            Ok(())
        }
    }
    /// Returns whether the given macro_id is in the frame stack.
    fn in_macro(&self, macro_id: usize) -> bool {
        for frame in &self.frames {
            let frame_macro_id = match *frame {
                Frame::SingleToken { macro_id, .. }
                | Frame::FuncMacro { macro_id, .. }
                | Frame::ObjectMacro { macro_id, .. } => macro_id,
                _ => continue,
            };

            if frame_macro_id == macro_id {
                return true;
            }
        }
        false
    }
}

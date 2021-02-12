// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        traveler::{
            CTravelerState,
            FrameStack,
            MacroKind,
        },
        CCompileEnv,
        CStringType,
        CToken,
        CTokenKind,
        CTokenKind::*,
        CTokenStack,
    },
    sync::Arc,
};

pub struct CTraveler {
    env: Arc<CCompileEnv>,
    frames: FrameStack,
}

impl CTraveler {
    pub fn new(env: Arc<CCompileEnv>) -> CTraveler {
        let frames = FrameStack::new(env.clone());
        // OPTIMIZATION: A different hasher may be more performant
        CTraveler { env, frames }
    }

    pub fn load_start(&mut self, tokens: Arc<CTokenStack>) {
        self.frames.load_start(tokens);
        // self.frames starts before the first token in the file.
        // This allows handling any preprocessor instructions at the start of the file.
        self.move_forward();
    }

    pub fn save_state(&self) -> CTravelerState {
        self.frames.save_state()
    }

    pub fn load_state(&mut self, state: CTravelerState) {
        self.frames.load_state(state);
    }

    pub fn head(&self) -> &CToken {
        self.frames.head()
    }

    pub fn move_forward(&mut self) -> &CToken {
        self.frames.move_forward();
        loop {
            if self.frames.is_token_joiner_next() {
                self.handle_joiner();
                continue;
            }

            match *self.frames.head().kind() {
                PreIf { link } => {
                    self.handle_if(link);
                },
                PreIfDef { link } => {
                    self.handle_if_def(true, link);
                },
                PreIfNDef { link } => {
                    self.handle_if_def(false, link);
                },
                PreElif { link } => {
                    if self.frames.should_chain_skip() {
                        self.frames.skip_to(link, true);
                    } else {
                        self.handle_if(link);
                    }
                },
                PreElse { link } => {
                    if self.frames.should_chain_skip() {
                        self.frames.skip_to(link, true);
                    } else {
                        self.ensure_end_of_preprocessor(true);
                    }
                },
                PreBlank => {
                    // Pre blank doesn't have a corresponding PreEnd
                    self.frames.move_forward();
                },
                PreEndIf => self.ensure_end_of_preprocessor(true),
                PreDefine => self.handle_define(),
                PreUndef => self.handle_undef(),
                PreLine => {
                    // TODO: Report warning that line can't be supported
                    self.skip_past_preprocessor();
                    eprintln!("#line cannot be supported");
                },
                PreInclude => self.handle_include(false),
                PreIncludeNext => self.handle_include(true),
                PreError => self.handle_message(true),
                PreWarning => self.handle_message(false),
                PreUnknown(ref _str) => {
                    unimplemented!("TODO: Error")
                },
                PrePragma => unimplemented!("#pragma isn't implemented yet."),
                Pragma => {
                    unimplemented!("_Pragma isn't implemented yet.")
                },
                Identifier(ref id) => match self.frames.should_handle_macro(id.uniq_id()) {
                    Some(handle) => self.frames.handle_macro(handle),
                    None => break,
                },
                Hash { .. } => {
                    unimplemented!("# isn't implemented yet.")
                },
                HashHash { .. } => {
                    unimplemented!("## isn't implemented yet.")
                },
                // It would be nice to return here, but borrow checker.
                _ => break,
            }
        }

        self.frames.head()
    }

    fn handle_joiner(&mut self) {
        let first_token = self.head().clone();
        self.frames.move_forward();
        let second_token = self.move_forward().clone();

        #[allow(clippy::pattern_type_mismatch)]
        let joined = match (first_token.kind(), second_token.kind()) {
            (LAngle, Colon) => LBracket { alt: true },
            (Colon, RAngle) => RBracket { alt: true },
            (LAngle, Percent) => LBrace { alt: true },
            (Percent, RAngle) => RBrace { alt: true },
            (Amp, Equal) => AmpEqual,
            (Amp, Amp) => AmpAmp,
            (Minus, LAngle) => Arrow,
            (Bang, Equal) => BangEqual,
            (Bar, Equal) => BarEqual,
            (Bar, Bar) => BarBar,
            (Carrot, Equal) => CarrotEqual,
            (Equal, Equal) => EqualEqual,
            (Percent, Colon) => Hash { alt: true },
            (Hash { .. }, Hash { .. }) => HashHash { alt: false },
            (Minus, Equal) => MinusEqual,
            (Minus, Minus) => MinusMinus,
            (LAngle, Equal) => LAngleEqual,
            (LAngle, LAngle) => LShift,
            (LShift, Equal) => LShiftEqual,
            (Percent, Equal) => PercentEqual,
            (Plus, Equal) => PlusEqual,
            (Plus, Plus) => PlusPlus,
            (RAngle, Equal) => RAngleEqual,
            (RAngle, RAngle) => RShift,
            (RShift, Equal) => RShiftEqual,
            (Slash, Equal) => SlashEqual,
            (Star, Equal) => StarEqual,
            (
                Identifier(ref id),
                String {
                    str_type: CStringType::Default,
                    is_char,
                    has_complex_escapes,
                    str_data,
                },
            ) => {
                if let Some(str_type) = self.env.cached_to_str_prefix().get(id) {
                    String {
                        str_type: *str_type,
                        is_char: *is_char,
                        has_complex_escapes: *has_complex_escapes,
                        str_data: str_data.clone(),
                    }
                } else {
                    // TODO: Report that id was not a valid string prefix
                    unimplemented!()
                }
            },
            (Number(num1), Number(num2)) | (Number(num1), Identifier(num2)) => {
                let mut num_combined = std::string::String::with_capacity(num1.len() + num2.len());
                num_combined.push_str(num1.string());
                num_combined.push_str(num2.string());
                let cached = self.env.cache().get_or_cache(num_combined.as_str());
                Number(cached)
            },
            (id1, id2) if id1.is_id_joinable() && id2.is_id_joinable() => {
                let id1 = id1.get_id_join_text();
                let id2 = id2.get_id_join_text();
                let mut id_combined = std::string::String::with_capacity(id1.len() + id2.len());
                id_combined.push_str(id1);
                id_combined.push_str(id2);
                let cached = self.env.cache().get_or_cache(id_combined.as_str());
                if let Some(keyword) = self.env.cached_to_keywords().get(&cached) {
                    keyword.clone()
                } else {
                    Identifier(cached)
                }
            },
            _ => {
                // TODO: Error about invalid token.
                return;
            },
        };

        // TODO: Calculate token length (or just point to the joiner)
        let joined_token = CToken::new_unknown(joined);

        self.frames.push_token(joined_token);
    }

    fn handle_if(&mut self, _link: usize) {
        // TODO: This may be messy since it needs order-of-operations
        unimplemented!("TODO:")
    }

    fn handle_if_def(&mut self, if_def: bool, link: usize) {
        let defined = match *self.frames.move_forward().kind() {
            Identifier(ref id) => {
                let macro_id = id.uniq_id();
                self.frames.has_macro(macro_id)
            },
            PreEnd => {
                // TODO: Report missing identifier.
                eprintln!("Missing identifier to ifdef/ifndef");
                self.frames.skip_to(link, false);
                return;
            },
            _ => {
                // TODO: Report mis-match.
                eprintln!("Expected identifier");
                self.skip_past_preprocessor();
                return;
            },
        };

        if defined != if_def {
            self.frames.skip_to(link, false);
            return;
        }

        self.ensure_end_of_preprocessor(true);
    }

    fn handle_define(&mut self) {
        let macro_id = match *self.frames.move_forward().kind() {
            Identifier(ref id) => id.uniq_id(),
            PreEnd => {
                // TODO: Report missing identifier.
                eprintln!("Missing identifier to define");
                self.frames.move_forward();
                return;
            },
            _ => {
                // TODO: Report mis-match.
                eprintln!("Expected identifier to define");
                self.skip_past_preprocessor();
                return;
            },
        };

        let head = self.frames.move_forward();
        match *head.kind() {
            PreEnd => {
                // TODO: Ensure the previous macro was empty (otherwise report warning)
                self.frames.add_macro(macro_id, MacroKind::Empty);
                self.frames.move_forward();
            },
            LParen if !head.whitespace_before() => {
                self.handle_function_macro(macro_id);
            },
            _ => {
                self.handle_object_macro(macro_id);
            },
        }
    }

    fn handle_function_macro(&mut self, macro_id: usize) {
        let mut params = Vec::new();
        let mut var_arg = None;
        loop {
            match *self.frames.move_forward().kind() {
                Identifier(ref id) => params.push(id.uniq_id()),
                DotDotDot => {
                    var_arg = Some(self.env.cache().get_or_cache("__VA_ARGS__").uniq_id());
                    break;
                },
                RParen => break,
                _ => {
                    // TODO: Report token not valid in func macro params
                    self.skip_past_preprocessor();
                    return;
                },
            }

            match *self.frames.move_forward().kind() {
                Comma => continue,
                DotDotDot => {
                    var_arg = Some(params.pop().unwrap());
                    break;
                },
                RParen => break,
                Identifier(_) => {
                    // TODO: Report missing , or ) between parameters
                    self.skip_past_preprocessor();
                    return;
                },
                _ => {
                    // TODO: Report token not valid in func macro params
                    self.skip_past_preprocessor();
                    return;
                },
            }
        }

        match *self.frames.head().kind() {
            RParen => {
                self.frames.move_forward();
            },
            Comma => {
                // TODO: Report that ) must follow var-arg parameter. Cannot have another parameter.
                self.skip_past_preprocessor();
                return;
            },
            _ => {
                // TODO: Report ) must follow var-arg parameter.
                self.skip_past_preprocessor();
                return;
            },
        }

        let (file_id, index) = self.frames.get_file_index();
        let length = self.skip_past_preprocessor();
        self.frames.add_macro(macro_id, MacroKind::FuncMacro {
            file_id,
            index,
            end: index + length,
            param_ids: params,
            var_arg,
        });
    }

    fn handle_object_macro(&mut self, macro_id: usize) {
        if matches!(
            self.frames.preview_next_kind(false),
            Some(&CTokenKind::PreEnd)
        ) {
            // TODO: Ensure the previous macro is the same (otherwise report warning)
            let token = self.frames.head().clone();
            self.frames.add_macro(macro_id, MacroKind::SingleToken { token });
            // Move onto the PreEnd token
            self.frames.move_forward();
            // Move past the PreEnd token
            self.frames.move_forward();
        } else {
            let (file_id, index) = self.frames.get_file_index();
            let length = self.skip_past_preprocessor();
            // TODO: Ensure the previous macro was the same (otherwise report warning)
            self.frames.add_macro(macro_id, MacroKind::ObjectMacro {
                index,
                file_id,
                end: index + length,
            });
        }
    }

    fn handle_undef(&mut self) {
        match *self.frames.move_forward().kind() {
            Identifier(ref id) => {
                let macro_id = id.uniq_id();
                self.frames.remove_macro(macro_id);
            },
            PreEnd => {
                // TODO: Report missing identifier.
                eprintln!("Missing identifier to undef");
                self.frames.move_forward();
                return;
            },
            _ => {
                // TODO: Report mis-match.
                eprintln!("Expected identifier to undef");
                self.skip_past_preprocessor();
                return;
            },
        };

        self.ensure_end_of_preprocessor(true);
    }

    fn handle_include(&mut self, _include_next: bool) {
        // We use self.move_forward to allow for macros to be decoded.
        let inc_file = match *self.move_forward().kind() {
            IncludePath { ref path, .. } => {
                let path = path.clone();
                self.ensure_end_of_preprocessor(false);
                self.frames.get_include_ref(path)
            },
            String { ref str_data, .. } => {
                // TODO:
                eprintln!(
                    "Indirection with quotes is not yet supported. Included: {}",
                    str_data
                );
                self.ensure_end_of_preprocessor(true);
                return;
            },
            LAngle => {
                eprintln!("Indirection <> include is not supported currently.");
                self.skip_past_preprocessor();
                return;
            },
            PreEnd => {
                // TODO: Report missing include
                eprintln!("Missing include");
                self.frames.move_forward();
                return;
            },
            _ => {
                // TODO: Report mis-match.
                eprintln!("Expected include");
                self.skip_past_preprocessor();
                return;
            },
        };

        if self.frames.push_include(inc_file).is_err() {
            // TODO: Report missing include error.
            eprintln!("Missing include. ID: {}", inc_file);
        }
    }

    fn handle_message(&mut self, is_error: bool) {
        match *self.frames.move_forward().kind() {
            Message(ref text) => {
                eprintln!(
                    "{}: {}",
                    if is_error { "ERROR " } else { "WARNING" },
                    text
                );
                self.ensure_end_of_preprocessor(true);
            },
            PreEnd => {
                // TODO: Report missing identifier.
                eprintln!("Report error with no message");
                self.skip_past_preprocessor();
            },
            _ => panic!(
                "Message preprocessor instructions should only be followed by Message or EndPreprocessor."
            ),
        };
    }

    fn ensure_end_of_preprocessor(&mut self, move_past_end: bool) {
        if let PreEnd = *self.frames.move_forward().kind() {
            if move_past_end {
                self.frames.move_forward();
            }
        } else {
            // TODO: Report extra token
            eprintln!("Extra tokens");
            self.skip_past_preprocessor();
        }
    }

    fn skip_past_preprocessor(&mut self) -> usize {
        // We start with 1 because we'll always move at least 1 token forward.
        let mut count = 1usize;
        while !matches!(self.frames.move_forward().kind(), &PreEnd) {
            count += 1;
        }
        // Move past the PreEnd token.
        self.frames.move_forward();
        count
    }
}

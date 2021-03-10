// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        traveler::{
            FrameStack,
            IfEvaluator,
            IfParser,
            MacroKind,
            TravelerError,
            TravelerState,
        },
        CompileEnv,
        FileTokens,
        Keyword,
        StringEncoding,
        Token,
        TokenKind,
        TokenKind::*,
    },
    error::{
        MayUnwind,
        Unwind,
    },
    sync::Arc,
    util::{
        CachedString,
        StringBuilder,
    },
};

type Error = crate::c::traveler::TravelerErrorKind;

pub struct Traveler<OnError>
where OnError: FnMut(TravelerError) -> bool
{
    pub(super) env: Arc<CompileEnv>,
    pub(super) frames: FrameStack,
    str_builder: StringBuilder,
    on_error: OnError,
}

impl<OnError> Traveler<OnError>
where OnError: FnMut(TravelerError) -> bool
{
    pub fn new(env: Arc<CompileEnv>, on_error: OnError) -> Self {
        let frames = FrameStack::new(env.clone());
        // OPTIMIZATION: A different hasher may be more performant
        Traveler {
            env,
            frames,
            str_builder: StringBuilder::new(),
            on_error,
        }
    }

    pub fn load_start(&mut self, tokens: Arc<FileTokens>) -> MayUnwind<()> {
        self.frames.load_start(tokens);
        // self.frames starts before the first token in the file.
        // This allows handling any preprocessor instructions at the start of the file.
        self.move_forward()?;
        Ok(())
    }

    pub fn save_state(&self) -> TravelerState {
        self.frames.save_state()
    }

    pub fn load_state(&mut self, state: TravelerState) {
        self.frames.load_state(state);
    }

    pub fn head(&self) -> &Token {
        self.frames.head()
    }

    pub fn move_forward(&mut self) -> MayUnwind<&Token> {
        self.frames.move_forward();
        loop {
            if self.frames.is_token_joiner_next() {
                self.handle_joiner()?;
                continue;
            }

            let head = self.frames.head();
            match *head.kind() {
                PreIf { link } => {
                    let head = head.clone();
                    self.handle_if(head, link)?;
                },
                PreIfDef { link } | PreIfNDef { link } => {
                    let head = head.clone();
                    self.handle_if_def(head, link)?;
                },
                PreElif { link } => {
                    if self.frames.should_chain_skip() {
                        self.frames.skip_to(link, true);
                    } else {
                        let head = head.clone();
                        self.handle_if(head, link)?;
                    }
                },
                PreElse { link } => {
                    let should_chain_skip = self.frames.should_chain_skip();
                    self.ensure_end_of_preprocessor(Error::ElseExtraTokens)?;
                    if should_chain_skip {
                        self.frames.skip_to(link, true);
                    }
                },
                PreBlank => {
                    // Pre blank doesn't have a corresponding PreEnd
                    self.frames.move_forward();
                },
                PreEndIf => self.ensure_end_of_preprocessor(Error::EndIfExtraTokens)?,
                PreDefine => self.handle_define()?,
                PreUndef => self.handle_undef()?,
                PreLine => {
                    self.report_error(Error::UnsupportableLinePreprocessor)?;
                    self.skip_past_preprocessor();
                },
                PreInclude => self.handle_include(false)?,
                PreIncludeNext => self.handle_include(true)?,
                PreError => self.handle_message(true)?,
                PreWarning => self.handle_message(false)?,
                PreUnknown(ref str) => {
                    let error = Error::UnknownPreprocessor(str.clone());
                    self.report_error(error)?;
                    self.skip_past_preprocessor();
                },
                PrePragma => {
                    self.report_error(Error::Unimplemented("#pragma"))?;
                    unreachable!();
                },
                Keyword(Keyword::Pragma, ..) => {
                    self.report_error(Error::Unimplemented("_Pragma"))?;
                    unreachable!();
                },
                ref token if token.is_definable() => {
                    let definable_id = token.get_definable_id();
                    if let Some(handle) = self.frames.should_handle_macro(definable_id) {
                        self.frames.handle_macro(handle, &mut self.on_error)?;
                    } else {
                        break;
                    }
                },
                LexerError(index) => {
                    let error = self.frames.get_current_file().errors()[index].clone();
                    self.report_error(error.into())?;
                    self.frames.move_forward();
                },
                Hash { .. } => {
                    self.report_error(Error::StrayHash)?;
                    self.frames.move_forward();
                },
                HashHash { .. } => {
                    self.report_error(Error::StrayHashHash)?;
                    self.frames.move_forward();
                },
                At => {
                    self.report_error(Error::StrayAtSign)?;
                    self.frames.move_forward();
                },
                Backslash => {
                    self.report_error(Error::StrayBackslash)?;
                    self.frames.move_forward();
                },
                // It would be nice to return here, but borrow checker.
                _ => break,
            }
        }

        Ok(self.frames.head())
    }

    fn move_slightly_forward(&mut self) -> MayUnwind<&Token> {
        while let LexerError(index) = *self.frames.move_forward().kind() {
            let error = self.frames.get_current_file().errors()[index].clone();
            self.report_error(error.into())?;
        }

        Ok(self.frames.head())
    }

    fn handle_if(&mut self, if_token: Token, link: usize) -> MayUnwind<()> {
        self.move_forward()?;
        let mut expr = match IfParser::create_and_parse(self, &if_token) {
            Ok(expr) => expr,
            Err(Unwind::Block) => {
                // We failed to parse the if condition, so we assume it's false.
                self.frames.skip_to(link, false);
                return Ok(());
            },
            Err(Unwind::Fatal) => return Err(Unwind::Fatal),
        };
        // Move past the PreEnd token.
        self.move_forward()?;
        match IfEvaluator::calc(&mut expr, if_token, |err| self.report_error(err)) {
            Ok(true) => Ok(()),
            Ok(false) | Err(Unwind::Block) => {
                self.frames.skip_to(link, false);
                Ok(())
            },
            Err(Unwind::Fatal) => Err(Unwind::Fatal),
        }
    }

    fn handle_if_def(&mut self, if_def: Token, link: usize) -> MayUnwind<()> {
        let is_ifdef = matches!(if_def.kind(), &PreIfDef { .. });
        let defined = match *self.move_slightly_forward()?.kind() {
            ref token if token.is_definable() => {
                let macro_id = token.get_definable_id();
                self.frames.has_macro(macro_id)
            },
            _ => {
                let error = Error::IfDefExpectedId(if_def, self.frames.head().clone());
                let result = self.report_error(error);
                self.frames.skip_to(link, false);
                return result;
            },
        };

        self.ensure_end_of_preprocessor(Error::IfDefExtraTokens(if_def))?;
        if defined != is_ifdef {
            self.frames.skip_to(link, false);
        }
        Ok(())
    }

    fn handle_define(&mut self) -> MayUnwind<()> {
        let macro_id = match *self.move_slightly_forward()?.kind() {
            ref token if token.is_definable() => token.get_definable_id(),
            _ => {
                let error = Error::DefineExpectedId(self.frames.head().clone());
                let result = self.report_error(error);
                self.skip_past_preprocessor();
                return result;
            },
        };

        let head = self.move_slightly_forward()?;
        match *head.kind() {
            PreEnd => {
                // TODO: Ensure the previous macro was empty (otherwise report warning)
                self.frames.add_macro(macro_id, MacroKind::Empty);
                self.frames.move_forward();
                Ok(())
            },
            LParen if !head.whitespace_before() => self.handle_function_macro(macro_id),
            _ => self.handle_object_macro(macro_id),
        }
    }

    fn handle_function_macro(&mut self, macro_id: usize) -> MayUnwind<()> {
        let mut params = Vec::new();
        let mut var_arg = None;
        loop {
            match *self.move_slightly_forward()?.kind() {
                ref token if token.is_definable() => params.push(token.get_definable_id()),
                DotDotDot => {
                    var_arg = Some(self.env.cache().get_or_cache("__VA_ARGS__").uniq_id());
                    self.frames.move_forward();
                    break;
                },
                RParen => break,
                PreEnd => {
                    let result = self.report_error(Error::DefineFuncEndBeforeEndOfArgs);
                    self.frames.move_forward();
                    return result;
                },
                _ => {
                    let error = Error::DefineFuncExpectedArg(self.frames.head().clone());
                    let result = self.report_error(error);
                    self.skip_past_preprocessor();
                    return result;
                },
            }

            match *self.move_slightly_forward()?.kind() {
                Comma => continue,
                DotDotDot => {
                    var_arg = Some(params.pop().unwrap());
                    self.frames.move_forward();
                    break;
                },
                RParen => break,
                PreEnd => {
                    let result = self.report_error(Error::DefineFuncEndBeforeEndOfArgs);
                    self.frames.move_forward();
                    return result;
                },
                _ => {
                    let error = Error::DefineFuncExpectedSeparator(self.frames.head().clone());
                    let result = self.report_error(error);
                    self.skip_past_preprocessor();
                    return result;
                },
            }
        }

        match *self.frames.head().kind() {
            RParen => {
                self.frames.move_forward();
            },
            _ => {
                let error = Error::DefineFuncExpectedEndOfArgs(self.frames.head().clone());
                let result = self.report_error(error);
                self.skip_past_preprocessor();
                return result;
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

        Ok(())
    }

    fn handle_object_macro(&mut self, macro_id: usize) -> MayUnwind<()> {
        if matches!(
            self.frames.preview_next_kind(false),
            Some(&TokenKind::PreEnd)
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

        Ok(())
    }

    fn handle_undef(&mut self) -> MayUnwind<()> {
        match *self.move_slightly_forward()?.kind() {
            ref token if token.is_definable() => {
                let macro_id = token.get_definable_id();
                self.frames.remove_macro(macro_id);
            },
            _ => {
                let error = Error::UndefExpectedId(self.frames.head().clone());
                let result = self.report_error(error);
                self.skip_past_preprocessor();
                return result;
            },
        };

        self.ensure_end_of_preprocessor(Error::UndefExtraTokens)?;
        Ok(())
    }

    fn handle_include(&mut self, _include_next: bool) -> MayUnwind<()> {
        // We use self.move_forward to allow for macros to be used.
        let inc_file = match *self.move_forward()?.kind() {
            IncludePath { ref path, inc_type } => {
                let path = path.clone();
                if let Some(inc_file) = self.frames.get_include_ref(&path) {
                    inc_file
                } else {
                    let error = Error::IncludeNotFound(inc_type, path);
                    let result = self.report_error(error);
                    self.skip_past_preprocessor();
                    return result;
                }
            },
            String { is_char: false, .. } => {
                self.report_error(Error::Unimplemented("Include indirection with quotes"))?;
                unreachable!()
            },
            LAngle => {
                self.report_error(Error::Unimplemented("Include indirection with <>"))?;
                unreachable!()
            },
            _ => {
                let error = Error::IncludeExpectedPath(self.head().clone());
                let result = self.report_error(error);
                self.skip_past_preprocessor();
                return result;
            },
        };

        if !matches!(*self.frames.move_forward().kind(), PreEnd) {
            self.report_error(Error::IncludeExtraTokens)?;
            while !matches!(*self.frames.move_forward().kind(), PreEnd) {}
        }

        if self.frames.push_include(inc_file).is_err() {
            self.report_error(Error::MissingIncludeId(inc_file))
        } else {
            Ok(())
        }
    }

    fn handle_message(&mut self, is_error: bool) -> MayUnwind<()> {
        let state = self.save_state();
        let message = match *self.move_slightly_forward()?.kind() {
            Message(ref text) => {
                let text = text.clone();
                // The next token *should* be a PreEnd token.
                self.skip_past_preprocessor();
                Some(text)
            },
            PreEnd => {
                self.frames.move_forward();
                None
            },
            _ => {
                let error = Error::Unreachable(
                    "Message preprocessor instructions should only be followed by Message or PreEnd token.",
                );
                self.report_error(error)?;
                unreachable!()
            },
        };

        let error_kind = if is_error {
            Error::ErrorPreprocessor(message)
        } else {
            Error::WarningPreprocessor(message)
        };

        self.report_error_with_state(error_kind, state)
    }

    fn handle_joiner(&mut self) -> MayUnwind<()> {
        self.str_builder.clear();
        let first_token = self.head().clone();
        let join_loc = self.frames.move_forward().loc();
        let second_token = self.move_slightly_forward()?.clone();

        if let Some(joined) = self.attempt_join(&first_token, &second_token) {
            let joined_token = Token::new(join_loc, true, joined);
            self.frames.push_token(joined_token);
            Ok(())
        } else {
            // TODO: Make the recovery after this error better by having first_token get processed
            // as if it wasn't joined. (Right now this just skips first_token straight to second_token).
            let error = Error::InvalidJoin(first_token, join_loc, second_token);
            self.report_error(error)
        }
    }

    fn attempt_join(&mut self, first_token: &Token, second_token: &Token) -> Option<TokenKind> {
        #[allow(clippy::pattern_type_mismatch)]
        let joined = match (first_token.kind(), second_token.kind()) {
            (LAngle, Colon) => LBracket { alt: true },
            (Colon, RAngle) => RBracket { alt: true },
            (LAngle, Percent) => LBrace { alt: true },
            (Percent, RAngle) => RBrace { alt: true },
            (Amp, Equal) => AmpEqual,
            (Amp, Amp) => AmpAmp,
            (Minus, RAngle) => Arrow,
            (Bang, Equal) => BangEqual,
            (Bar, Equal) => BarEqual,
            (Bar, Bar) => BarBar,
            (Carrot, Equal) => CarrotEqual,
            (Equal, Equal) => EqualEqual,
            (Percent, Colon) => Hash { alt: true },
            (Hash { alt: false }, Hash { alt: false }) => HashHash { alt: false },
            (Hash { alt: true }, Hash { alt: true }) => HashHash { alt: true },
            (Minus, Equal) => MinusEqual,
            (Minus, Minus) => MinusMinus,
            (LAngle, Equal) => LAngleEqual,
            (LAngle, LAngle) => LShift,
            (LAngle, LAngleEqual) => LShiftEqual,
            (LShift, Equal) => LShiftEqual,
            (Percent, Equal) => PercentEqual,
            (Plus, Equal) => PlusEqual,
            (Plus, Plus) => PlusPlus,
            (RAngle, Equal) => RAngleEqual,
            (RAngle, RAngle) => RShift,
            (RAngle, RAngleEqual) => RShiftEqual,
            (RShift, Equal) => RShiftEqual,
            (Slash, Equal) => SlashEqual,
            (Star, Equal) => StarEqual,
            (
                Identifier(ref id),
                String {
                    encoding: StringEncoding::Default,
                    is_char,
                    has_escapes,
                    str_data,
                },
            ) => {
                if let Some(str_type) = self.env.cached_to_str_prefix().get(id) {
                    String {
                        encoding: *str_type,
                        is_char: *is_char,
                        has_escapes: *has_escapes,
                        str_data: str_data.clone(),
                    }
                } else {
                    return None;
                }
            },
            (part1, part2) if part1.is_number_joinable_with(part2) => {
                let digits = self.join_and_cache(part1.text(), part2.text());
                Number(digits)
            },
            (id1, id2) if id1.is_id_joinable_with(id2) => {
                let cached = self.join_and_cache(id1.text(), id2.text());
                if let Some(keyword) = self.env.cached_to_keywords().get(&cached) {
                    Keyword(*keyword, cached.uniq_id())
                } else {
                    Identifier(cached)
                }
            },
            _ => return None,
        };
        Some(joined)
    }

    fn join_and_cache(&mut self, s1: &str, s2: &str) -> CachedString {
        self.str_builder.clear();
        self.str_builder.reserve(s1.len() + s2.len());
        self.str_builder.append_str(s1);
        self.str_builder.append_str(s2);
        self.env.cache().get_or_cache(self.str_builder.current())
    }

    fn ensure_end_of_preprocessor(&mut self, error: Error) -> MayUnwind<()> {
        if let PreEnd = *self.frames.move_forward().kind() {
            self.frames.move_forward();
            Ok(())
        } else {
            let result = self.report_error(error);
            self.skip_past_preprocessor();
            result
        }
    }

    pub(super) fn report_error(&mut self, v: Error) -> MayUnwind<()> {
        self.report_error_with_state(v, self.save_state())
    }

    fn report_error_with_state(&mut self, v: Error, state: TravelerState) -> MayUnwind<()> {
        use crate::error::CodedError;
        let mut fatal = v.severity().is_fatal();
        let error = TravelerError { kind: v, state };

        fatal |= (self.on_error)(error);

        if fatal { Err(Unwind::Fatal) } else { Ok(()) }
    }

    fn skip_past_preprocessor(&mut self) -> usize {
        if matches!(self.frames.head().kind(), &PreEnd) {
            self.frames.move_forward();
            return 0;
        }
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

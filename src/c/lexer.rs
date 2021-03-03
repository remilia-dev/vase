// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fs::File;
use std::path::Path;

use unicode_normalization::UnicodeNormalization;

use crate::{
    c::{
        token::*,
        CompileEnv,
        FileReader,
        FileTokens,
        LexerError,
        LexerErrorKind,
    },
    sync::Arc,
    util::{
        CachedString,
        FileId,
        SourceLocation,
        StringBuilder,
    },
};

pub trait IncludeCallback = FnMut(IncludeType, &CachedString, &Option<Arc<Path>>) -> Option<FileId>;

pub struct Lexer<'a, OnInclude: IncludeCallback> {
    env: &'a CompileEnv,
    include_callback: OnInclude,
    reader: FileReader,
    str_builder: StringBuilder,
    norm_buffer: StringBuilder,
    link_stack: Vec<usize>,
}

impl<'a, OnInclude: IncludeCallback> Lexer<'a, OnInclude> {
    pub fn new(env: &'a CompileEnv, include_callback: OnInclude) -> Self {
        Lexer {
            env,
            include_callback,
            reader: FileReader::new(),
            str_builder: StringBuilder::with_capacity(30),
            norm_buffer: StringBuilder::with_capacity(30),
            link_stack: Vec::with_capacity(5),
        }
    }

    /// Lexes the file at the given path and produces a stack of all the tokens.
    /// # Errors
    /// Only *fatal* lexer errors are returned. Other errors (such as improperly ended strings)
    /// are reported using a [LexerError](TokenKind::LexerError) token.
    pub fn lex_file(&mut self, file_id: FileId, file_path: Arc<Path>) -> FileTokens {
        // The scope is here to free file resources early.
        {
            let file = match File::open(&file_path) {
                Err(error) => {
                    return FileTokens::new_error(file_id, Some(file_path), error);
                },
                Ok(f) => f,
            };

            if file.metadata().unwrap().len() == 0 {
                // Can't memory map a 0-byte file.
                return FileTokens::new_empty(file_id, Some(file_path));
            }

            // OPTIMIZATION: Would getting away from memory mapping be faster?
            // TODO: Lock the file that is being mapped. This would prevent the memory map from changing under us.
            // It would also allow this to be truly safe.
            let mmap = match unsafe { memmap2::MmapOptions::new().map(&file) } {
                Err(error) => {
                    return FileTokens::new_error(file_id, Some(file_path), error);
                },
                Ok(m) => m,
            };

            if let Some(error) = self.reader.load_bytes(file_id, &mmap) {
                return FileTokens::new_error(file_id, Some(file_path), error);
            }
        }

        self.lex(file_id, Some(file_path))
    }

    pub fn lex_bytes(&mut self, file_id: FileId, bytes: &[u8]) -> FileTokens {
        if let Some(error) = self.reader.load_bytes(file_id, bytes) {
            return FileTokens::new_error(file_id, None, error);
        }
        self.lex(file_id, None)
    }

    fn lex(&mut self, file_id: FileId, path: Option<Arc<Path>>) -> FileTokens {
        LexerState::create_and_lex(file_id, path, self)
    }
}

#[derive(PartialEq)]
#[repr(u8)]
enum CLexerMode {
    Normal,
    Preprocessor,
    Include { next: bool },
    Message,
}

struct LexerState<'a, OnInclude: IncludeCallback> {
    mode: CLexerMode,
    at_start_of_line: bool,
    have_skipped_whitespace: bool,
    start_location: SourceLocation,
    tokens: FileTokens,
    env: &'a CompileEnv,
    include_callback: &'a mut OnInclude,
    reader: &'a mut FileReader,
    str_builder: &'a mut StringBuilder,
    norm_buffer: &'a mut StringBuilder,
    link_stack: &'a mut Vec<usize>,
}

impl<'a, OnInclude: IncludeCallback> LexerState<'a, OnInclude> {
    fn create_and_lex(
        file_id: FileId,
        path: Option<Arc<Path>>,
        shared_data: &'a mut Lexer<'_, OnInclude>,
    ) -> FileTokens {
        LexerState {
            mode: CLexerMode::Normal,
            at_start_of_line: true,
            have_skipped_whitespace: false,
            start_location: SourceLocation::new_first_byte(file_id),
            tokens: FileTokens::new(file_id, path),
            env: shared_data.env,
            include_callback: &mut shared_data.include_callback,
            reader: &mut shared_data.reader,
            str_builder: &mut shared_data.str_builder,
            norm_buffer: &mut shared_data.norm_buffer,
            link_stack: &mut shared_data.link_stack,
        }
        .lex()
    }

    #[must_use]
    fn lex(mut self) -> FileTokens {
        loop {
            self.have_skipped_whitespace |= self.reader.skip_most_whitespace();

            let (character, location) = match self.reader.front_location() {
                Some((char, location)) => (char, location),
                None => {
                    self.end_line();
                    break;
                },
            };
            self.start_location = location;

            match character {
                '/' if self.reader.move_forward_if_next('/') => self.lex_comment(false),
                '/' if self.reader.move_forward_if_next('*') => self.lex_comment(true),
                '\n' => self.end_line(),
                c if matches!(self.mode, CLexerMode::Message) => self.lex_message(c),
                '"' | '<' if matches!(self.mode, CLexerMode::Include { .. }) => {
                    self.lex_include(character)
                },
                '\'' | '"' => self.lex_string(StringEncoding::Default, character == '\''),
                c if r"~!@#%^&*()[]{}-+=:;\|,.<>/?".contains(c) => self.lex_symbol(c),
                c if c.is_ascii_digit() => self.lex_number(false, c),
                c => self.lex_identifier(c),
            };
        }

        let eof_token = Token::new(self.reader.location(), false, TokenKind::Eof);
        self.tokens.append(eof_token);

        self.tokens.finalize();
        self.tokens
    }

    // This function is long just due to the various combinations. Splitting it up would be less clear.
    #[allow(clippy::too_many_lines)]
    fn lex_symbol(&mut self, first_char: char) {
        // NOTE: Some of the branches here need a move forward (to get past the symbol)
        // while others don't. I couldn't figure out a nice way to handle this without
        // exploding the line count. For now, each branch also returns a boolean that
        // signals whether a move forward is required (true) or not (false).
        let (move_forward, kind) = match first_char {
            // TODO: Add double [[ and ]] support for C2X attributes
            '[' => (true, TokenKind::LBracket { alt: false }),
            ']' => (true, TokenKind::RBracket { alt: false }),
            '(' => (true, TokenKind::LParen),
            ')' => (true, TokenKind::RParen),
            '{' => (true, TokenKind::LBrace { alt: false }),
            '}' => (true, TokenKind::RBrace { alt: false }),
            '.' => match self.reader.move_forward() {
                Some('.') => {
                    if self.reader.move_forward_if_next('.') {
                        self.reader.move_forward();
                        (false, TokenKind::DotDotDot)
                    } else {
                        (false, TokenKind::Dot)
                    }
                },
                Some(c) if c.is_ascii_digit() => return self.lex_number(true, c),
                _ => (false, TokenKind::Dot),
            },
            '&' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::AmpEqual),
                Some('&') => (true, TokenKind::AmpAmp),
                _ => (false, TokenKind::Amp),
            },
            '*' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::StarEqual),
                _ => (false, TokenKind::Star),
            },
            '+' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::PlusEqual),
                Some('+') => (true, TokenKind::PlusPlus),
                _ => (false, TokenKind::Plus),
            },
            '-' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::MinusEqual),
                Some('-') => (true, TokenKind::MinusMinus),
                Some('>') => (true, TokenKind::Arrow),
                _ => (false, TokenKind::Minus),
            },
            '~' => (true, TokenKind::Tilde),
            '!' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::BangEqual),
                _ => (false, TokenKind::Bang),
            },
            '/' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::SlashEqual),
                // NOTE: Comments should have been handled in the main match in self.lex
                _ => (false, TokenKind::Slash),
            },
            '%' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::PercentEqual),
                Some('>') => (true, TokenKind::RBrace { alt: true }),
                Some(':') => match self.reader.move_forward() {
                    Some('%') if self.reader.move_forward_if_next(':') => {
                        (true, TokenKind::HashHash { alt: true })
                    },
                    _ => return self.lex_preprocessor(true),
                },
                _ => (false, TokenKind::Percent),
            },
            '<' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::LAngleEqual),
                Some('<') => match self.reader.move_forward() {
                    Some('=') => (true, TokenKind::LShiftEqual),
                    _ => (false, TokenKind::LShift),
                },
                Some('%') => (true, TokenKind::LBrace { alt: true }),
                Some(':') => (true, TokenKind::LBracket { alt: true }),
                _ => (false, TokenKind::LAngle),
            },
            '>' => match self.reader.move_forward() {
                Some('>') => match self.reader.move_forward() {
                    Some('=') => (true, TokenKind::RShiftEqual),
                    _ => (false, TokenKind::RShift),
                },
                Some('=') => (true, TokenKind::RAngleEqual),
                _ => (false, TokenKind::RAngle),
            },
            '=' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::EqualEqual),
                _ => (false, TokenKind::Equal),
            },
            '^' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::CarrotEqual),
                _ => (false, TokenKind::Carrot),
            },
            '|' => match self.reader.move_forward() {
                Some('=') => (true, TokenKind::BarEqual),
                Some('|') => (true, TokenKind::BarBar),
                _ => (false, TokenKind::Bar),
            },
            '?' => (true, TokenKind::QMark),
            ':' => match self.reader.move_forward() {
                Some('>') => (true, TokenKind::RBracket { alt: true }),
                _ => (false, TokenKind::Colon),
            },
            ';' => (true, TokenKind::Semicolon),
            ',' => (true, TokenKind::Comma),
            '#' => match self.reader.move_forward() {
                Some('#') => (true, TokenKind::HashHash { alt: false }),
                _ => return self.lex_preprocessor(false),
            },
            '@' => (true, TokenKind::At),
            '\\' => (true, TokenKind::Backslash),
            _ => unreachable!(
                "c_lex_symbol should never lex starting on the character {}.",
                first_char
            ),
        };

        if move_forward {
            self.reader.move_forward();
        }
        self.add_token(kind);
    }

    fn lex_preprocessor(&mut self, alt_start: bool) {
        if self.mode == CLexerMode::Preprocessor || !self.at_start_of_line {
            return self.add_token(TokenKind::Hash { alt: alt_start });
        }

        // The # or %: should have already been passed
        self.reader.skip_most_whitespace();

        let first_char = match self.reader.front() {
            Some(c) if c != '\n' => c,
            // If the EOF or a new line is next, we just want to return a blank preprocessor instruction.
            _ => return self.add_token(TokenKind::PreBlank),
        };

        let pre_id = self.read_cached_identifier(first_char);
        let pre_type = match self.env.cached_to_preprocessor().get(&pre_id) {
            Some(pre_type) => pre_type.clone(),
            None => {
                self.mode = CLexerMode::Preprocessor;
                return self.add_token(TokenKind::PreUnknown(pre_id));
            },
        };

        if pre_type.ends_a_link() {
            let curr_index = self.tokens.len();
            match self.link_stack.pop() {
                Some(index) => self.tokens[index].kind_mut().set_link(curr_index),
                None => {
                    let location = self.source_location();
                    let error = LexerError::new(location, LexerErrorKind::MissingCorrespondingIf);
                    self.tokens.add_error_token(error);
                },
            }
        }
        if pre_type.is_linking() {
            self.link_stack.push(self.tokens.len());
        }

        self.mode = match pre_type {
            TokenKind::PreInclude => CLexerMode::Include { next: false },
            TokenKind::PreIncludeNext => CLexerMode::Include { next: true },
            TokenKind::PreError | TokenKind::PreWarning => CLexerMode::Message,
            _ => CLexerMode::Preprocessor,
        };

        self.add_token(pre_type)
    }

    fn lex_include(&mut self, include_start: char) {
        let mut inc_type = match include_start {
            '"' => IncludeType::IncludeLocal,
            '<' => IncludeType::IncludeSystem,
            _ => panic!(
                "lex_include should not be called starting with the character '{}'.",
                include_start
            ),
        };

        self.str_builder.clear();
        let mut correctly_ended = false;
        while let Some(char) = self.reader.move_forward() {
            match char {
                '\n' => break,
                '"' | '>' if inc_type.is_end_char(char) => {
                    correctly_ended = true;
                    self.reader.move_forward();
                    break;
                },
                c => self.str_builder.append_char(c),
            }
        }

        if !correctly_ended {
            let location = self.reader.location();
            let error = LexerError::new(location, LexerErrorKind::UnendedInclude);
            self.tokens.add_error_token(error);
        }

        if let CLexerMode::Include { next } = self.mode {
            if next {
                inc_type = IncludeType::IncludeNext;
            }
        }
        let path = self.env.cache().get_or_cache(self.str_builder.current());

        let inc_id = (self.include_callback)(inc_type, &path, &self.tokens.path());
        self.tokens.add_reference(&path, inc_id);

        self.add_token(TokenKind::IncludePath { inc_type, path })
    }

    fn lex_message(&mut self, first_char: char) {
        self.str_builder.clear();
        self.str_builder.append_char(first_char);
        while let Some(char) = self.reader.move_forward() {
            if char == '\n' {
                break;
            }
            self.str_builder.append_char(char)
        }

        self.mode = CLexerMode::Normal;
        self.add_token(TokenKind::Message(
            self.str_builder.current_as_box().into(),
        ))
    }

    fn lex_string(&mut self, encoding: StringEncoding, is_char: bool) {
        let opening_char = if is_char { '\'' } else { '"' };
        self.str_builder.clear();

        let mut ended_correctly = false;
        let mut has_escapes = false;
        while let Some(char) = self.reader.move_forward() {
            match char {
                '\\' => {
                    has_escapes = true;
                    self.str_builder.append_ascii(b'\\');
                    if let Some(c) = self.reader.move_forward() {
                        self.str_builder.append_char(c);
                    } else {
                        break;
                    }
                },
                '\n' => break,
                c if c == opening_char => {
                    self.reader.move_forward();
                    ended_correctly = true;
                    break;
                },
                c => self.str_builder.append_char(c),
            }
        }

        if !ended_correctly {
            let location = self.reader.location();
            let error = LexerError::new(location, super::LexerErrorKind::UnendedString);
            self.tokens.add_error_token(error);
        }

        self.add_token(TokenKind::String {
            str_data: Arc::new(self.str_builder.current_as_box()),
            encoding,
            has_escapes,
            is_char,
        })
    }

    fn lex_number(&mut self, dot_start: bool, first_char: char) {
        self.str_builder.clear();
        if dot_start {
            self.str_builder.append_ascii(b'.');
        }

        // NOTE: All characters in a number are ascii
        self.str_builder.append_ascii(first_char as u8);

        while let Some(char) = self.reader.move_forward() {
            match char {
                'e' | 'E' | 'p' | 'P' => {
                    self.str_builder.append_ascii(char as u8);
                    if self.reader.move_forward_if_next('-') {
                        self.str_builder.append_ascii(b'-');
                    } else if self.reader.move_forward_if_next('+') {
                        self.str_builder.append_ascii(b'+');
                    }
                },
                '.' | '_' => self.str_builder.append_ascii(char as u8),
                c if c.is_whitespace() | c.is_ascii_punctuation() => break,
                c => self.str_builder.append_char(c),
            }
        }

        let num_data = self.env.cache().get_or_cache(self.str_builder.current());
        self.add_token(TokenKind::Number(num_data));
    }

    fn lex_identifier(&mut self, first_char: char) {
        let cached = self.read_cached_identifier(first_char);

        if let Some(keyword) = self.env.cached_to_keywords().get(&cached) {
            return self.add_token(TokenKind::Keyword(*keyword, cached.uniq_id()));
        }

        if let Some(str_type) = self.env.cached_to_str_prefix().get(&cached).cloned() {
            let front_char = self.reader.front().unwrap_or('\0');
            if front_char == '"' || front_char == '\'' {
                return self.lex_string(str_type, front_char == '\'');
            }
        }

        self.add_token(TokenKind::Identifier(cached));
    }

    fn lex_comment(&mut self, multi_line: bool) {
        loop {
            let char = match self.reader.move_forward() {
                Some(cl) => cl,
                None => {
                    if multi_line {
                        let location = self.reader.location();
                        let error = LexerError::new(location, LexerErrorKind::UnendedComment);
                        self.tokens.add_error_token(error);
                    }
                    return;
                },
            };

            match char {
                '\n' if !multi_line => return,
                '*' if multi_line => {
                    if self.reader.move_forward_if_next('/') {
                        self.reader.move_forward();
                        self.have_skipped_whitespace = true;
                        return;
                    }
                },
                _ => {},
            }
        }
    }

    fn read_cached_identifier(&mut self, first_char: char) -> CachedString {
        self.str_builder.clear();
        self.str_builder.append_char(first_char);

        while let Some(char) = self.reader.move_forward() {
            match char {
                c if c.is_whitespace() => break,
                '_' => {},
                c if c.is_ascii_punctuation() => break,
                _ => {},
            }

            self.str_builder.append_char(char);
        }

        let identifier = if self.str_builder.is_ascii() {
            self.str_builder.current()
        } else {
            for c in self.str_builder.current().nfkc() {
                self.norm_buffer.append_char(c);
            }
            self.norm_buffer.current()
        };

        return self.env.cache().get_or_cache(identifier);
    }

    fn end_line(&mut self) {
        if self.mode != CLexerMode::Normal {
            self.mode = CLexerMode::Normal;
            self.tokens.append(Token::new(
                self.reader.location(),
                false,
                TokenKind::PreEnd,
            ));
        }
        self.at_start_of_line = true;
        self.have_skipped_whitespace = true;
        self.reader.move_forward();
    }

    fn add_token(&mut self, kind: TokenKind) {
        let location = self.source_location();
        let token = Token::new(location, self.have_skipped_whitespace, kind);
        self.tokens.append(token);
        self.at_start_of_line = false;
        self.have_skipped_whitespace = false;
    }

    fn source_location(&self) -> SourceLocation {
        let end = self.reader.previous_location();
        self.start_location
            .through(&end)
            .unwrap_or_else(|| self.start_location.clone())
    }
}

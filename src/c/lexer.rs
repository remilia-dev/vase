// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fs::File;
use std::path::Path;

use unicode_normalization::UnicodeNormalization;

use crate::{
    c::{
        file_reader::*,
        token::*,
        CCompileEnv,
        CLexerError,
        CTokenStack,
        FileId,
    },
    sync::Arc,
    util::{
        CachedString,
        SourceLocation,
        StringBuilder,
    },
};

pub type CIncludeCallback<'a> =
    &'a (dyn Send + Sync + Fn(CIncludeType, &CachedString, &Option<Arc<Path>>) -> Option<FileId>);

pub struct CLexer<'a> {
    env: &'a CCompileEnv,
    include_callback: CIncludeCallback<'a>,
    reader: CFileReader,
    str_builder: StringBuilder,
    norm_buffer: StringBuilder,
    link_stack: Vec<usize>,
}
impl<'a> CLexer<'a> {
    pub fn new(env: &'a CCompileEnv, include_callback: CIncludeCallback<'a>) -> CLexer<'a> {
        CLexer {
            env,
            include_callback,
            reader: CFileReader::new(),
            str_builder: StringBuilder::with_capacity(30),
            norm_buffer: StringBuilder::with_capacity(30),
            link_stack: Vec::with_capacity(5),
        }
    }

    /// Lexes the file at the given path and produces a stack of all the tokens.
    /// # Errors
    /// Only *fatal* lexer errors are returned. Other errors (such as improperly ended strings)
    /// are reported using a [LexerError](CTokenKind::LexerError) token.
    pub fn lex_file(&mut self, file_id: FileId, file_path: Arc<Path>) -> CTokenStack {
        // The scope is here to free file resources early.
        {
            let file = match File::open(&file_path) {
                Err(err) => {
                    let error = CLexerError::Io(err.into());
                    return CTokenStack::new_error(file_id, Some(file_path), error);
                },
                Ok(f) => f,
            };

            if file.metadata().unwrap().len() == 0 {
                // Can't memory map a 0-byte file.
                return CTokenStack::new_empty(file_id, Some(file_path));
            }

            // OPTIMIZATION: Would getting away from memory mapping be faster?
            // TODO: Lock the file that is being mapped. This would prevent the memory map from changing under us.
            // It would also allow this to be truly safe.
            let mmap = match unsafe { memmap2::MmapOptions::new().map(&file) } {
                Err(err) => {
                    let error = CLexerError::Io(err.into());
                    return CTokenStack::new_error(file_id, Some(file_path), error);
                },
                Ok(m) => m,
            };

            if let Some(err) = self.reader.load_bytes(file_id, &mmap) {
                let error = CLexerError::Utf8Decode(err);
                return CTokenStack::new_error(file_id, Some(file_path), error);
            }
        }

        self.lex(file_id, Some(file_path))
    }

    pub fn lex_bytes(&mut self, file_id: FileId, bytes: &[u8]) -> CTokenStack {
        if let Some(err) = self.reader.load_bytes(file_id, bytes) {
            return CTokenStack::new_error(file_id, None, CLexerError::Utf8Decode(err));
        }
        self.lex(file_id, None)
    }

    fn lex(&mut self, file_id: FileId, path: Option<Arc<Path>>) -> CTokenStack {
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

struct LexerState<'a> {
    mode: CLexerMode,
    at_start_of_line: bool,
    have_skipped_whitespace: bool,
    start_location: SourceLocation,
    tokens: CTokenStack,
    env: &'a CCompileEnv,
    include_callback: CIncludeCallback<'a>,
    reader: &'a mut CFileReader,
    str_builder: &'a mut StringBuilder,
    norm_buffer: &'a mut StringBuilder,
    link_stack: &'a mut Vec<usize>,
}

impl<'a> LexerState<'a> {
    fn create_and_lex(
        file_id: FileId,
        path: Option<Arc<Path>>,
        shared_data: &'a mut CLexer,
    ) -> CTokenStack {
        LexerState {
            mode: CLexerMode::Normal,
            at_start_of_line: true,
            have_skipped_whitespace: false,
            start_location: SourceLocation::new_first_byte(file_id),
            tokens: CTokenStack::new(file_id, path),
            env: shared_data.env,
            include_callback: shared_data.include_callback,
            reader: &mut shared_data.reader,
            str_builder: &mut shared_data.str_builder,
            norm_buffer: &mut shared_data.norm_buffer,
            link_stack: &mut shared_data.link_stack,
        }
        .lex()
    }

    #[must_use]
    fn lex(mut self) -> CTokenStack {
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
                '\'' | '"' => self.lex_string(CStringType::Default, character == '\''),
                c if r"~!@#%^&*()[]{}-+=:;\|,.<>/?".contains(c) => self.lex_symbol(c),
                c if c.is_ascii_digit() => self.lex_number(false, c),
                c => self.lex_identifier(c),
            };
        }

        self.tokens.append(CToken::new(
            self.reader.location(),
            false,
            CTokenKind::Eof,
        ));

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
            '[' => (true, CTokenKind::LBracket { alt: false }),
            ']' => (true, CTokenKind::RBracket { alt: false }),
            '(' => (true, CTokenKind::LParen),
            ')' => (true, CTokenKind::RParen),
            '{' => (true, CTokenKind::LBrace { alt: false }),
            '}' => (true, CTokenKind::RBrace { alt: false }),
            '.' => match self.reader.move_forward() {
                Some('.') => {
                    if self.reader.move_forward_if_next('.') {
                        self.reader.move_forward();
                        (false, CTokenKind::DotDotDot)
                    } else {
                        (false, CTokenKind::Dot)
                    }
                },
                Some(c) if c.is_ascii_digit() => return self.lex_number(true, c),
                _ => (false, CTokenKind::Dot),
            },
            '&' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::AmpEqual),
                Some('&') => (true, CTokenKind::AmpAmp),
                _ => (false, CTokenKind::Amp),
            },
            '*' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::StarEqual),
                _ => (false, CTokenKind::Star),
            },
            '+' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::PlusEqual),
                Some('+') => (true, CTokenKind::PlusPlus),
                _ => (false, CTokenKind::Plus),
            },
            '-' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::MinusEqual),
                Some('-') => (true, CTokenKind::MinusMinus),
                Some('>') => (true, CTokenKind::Arrow),
                _ => (false, CTokenKind::Minus),
            },
            '~' => (true, CTokenKind::Tilde),
            '!' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::BangEqual),
                _ => (false, CTokenKind::Bang),
            },
            '/' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::SlashEqual),
                // NOTE: Comments should have been handled in the main match in self.lex
                _ => (false, CTokenKind::Slash),
            },
            '%' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::PercentEqual),
                Some('>') => (true, CTokenKind::RBrace { alt: true }),
                Some(':') => match self.reader.move_forward() {
                    Some('%') if self.reader.move_forward_if_next(':') => {
                        (true, CTokenKind::HashHash { alt: true })
                    },
                    _ => return self.lex_preprocessor(true),
                },
                _ => (false, CTokenKind::Percent),
            },
            '<' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::LAngleEqual),
                Some('<') => match self.reader.move_forward() {
                    Some('=') => (true, CTokenKind::LShiftEqual),
                    _ => (false, CTokenKind::LShift),
                },
                Some('%') => (true, CTokenKind::LBrace { alt: true }),
                Some(':') => (true, CTokenKind::LBracket { alt: true }),
                _ => (false, CTokenKind::LAngle),
            },
            '>' => match self.reader.move_forward() {
                Some('>') => match self.reader.move_forward() {
                    Some('=') => (true, CTokenKind::RShiftEqual),
                    _ => (false, CTokenKind::RShift),
                },
                Some('=') => (true, CTokenKind::RAngleEqual),
                _ => (false, CTokenKind::RAngle),
            },
            '=' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::EqualEqual),
                _ => (false, CTokenKind::Equal),
            },
            '^' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::CarrotEqual),
                _ => (false, CTokenKind::Carrot),
            },
            '|' => match self.reader.move_forward() {
                Some('=') => (true, CTokenKind::BarEqual),
                Some('|') => (true, CTokenKind::BarBar),
                _ => (false, CTokenKind::Bar),
            },
            '?' => (true, CTokenKind::QMark),
            ':' => match self.reader.move_forward() {
                Some('>') => (true, CTokenKind::RBracket { alt: true }),
                _ => (false, CTokenKind::Colon),
            },
            ';' => (true, CTokenKind::Semicolon),
            ',' => (true, CTokenKind::Comma),
            '#' => match self.reader.move_forward() {
                Some('#') => (true, CTokenKind::HashHash { alt: false }),
                _ => return self.lex_preprocessor(false),
            },
            '@' => (true, CTokenKind::At),
            '\\' => (true, CTokenKind::Backslash),
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
            return self.add_token(CTokenKind::Hash { alt: alt_start });
        }

        // The # or %: should have already been passed
        self.reader.skip_most_whitespace();

        let first_char = match self.reader.front() {
            Some(c) if c != '\n' => c,
            // If the EOF or a new line is next, we just want to return a blank preprocessor instruction.
            _ => return self.add_token(CTokenKind::PreBlank),
        };

        let pre_id = self.read_cached_identifier(first_char);
        let pre_type = match self.env.cached_to_preprocessor().get(&pre_id) {
            Some(pre_type) => pre_type.clone(),
            None => {
                self.mode = CLexerMode::Preprocessor;
                return self.add_token(CTokenKind::PreUnknown(pre_id));
            },
        };

        if pre_type.ends_a_link() {
            let curr_index = self.tokens.len();
            match self.link_stack.pop() {
                Some(index) => self.tokens[index].kind_mut().set_link(curr_index),
                None => {
                    let location = self.source_location();
                    let error = CLexerError::MissingCorrespondingIf(location.clone());
                    self.tokens.add_error_token(location, error);
                },
            }
        }
        if pre_type.is_linking() {
            self.link_stack.push(self.tokens.len());
        }

        self.mode = match pre_type {
            CTokenKind::PreInclude => CLexerMode::Include { next: false },
            CTokenKind::PreIncludeNext => CLexerMode::Include { next: true },
            CTokenKind::PreError | CTokenKind::PreWarning => CLexerMode::Message,
            _ => CLexerMode::Preprocessor,
        };

        self.add_token(pre_type)
    }

    fn lex_include(&mut self, include_start: char) {
        let mut inc_type = match include_start {
            '"' => CIncludeType::IncludeLocal,
            '<' => CIncludeType::IncludeSystem,
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
            let error = CLexerError::UnendedInclude(location.clone());
            self.tokens.add_error_token(location, error);
        }

        if let CLexerMode::Include { next } = self.mode {
            if next {
                inc_type = CIncludeType::IncludeNext;
            }
        }
        let path = self.env.cache().get_or_cache(self.str_builder.current());

        let inc_id = (self.include_callback)(inc_type, &path, &self.tokens.path());
        self.tokens.add_reference(&path, inc_id);

        self.add_token(CTokenKind::IncludePath { inc_type, path })
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
        self.add_token(CTokenKind::Message(
            self.str_builder.current_as_box().into(),
        ))
    }

    fn lex_string(&mut self, str_type: CStringType, is_char: bool) {
        let opening_char = if is_char { '\'' } else { '"' };
        self.str_builder.clear();

        let mut ended_correctly = false;
        let mut has_complex_escapes = false;
        while let Some(char) = self.reader.move_forward() {
            match char {
                '\\' => {
                    let simple_escape = match self.reader.move_forward() {
                        Some('\'') => '\'',
                        Some('"') => '"',
                        Some('?') => '?',
                        Some('\\') => '\\',
                        Some('a') => '\x07',
                        Some('b') => '\x08',
                        Some('f') => '\x0C',
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('v') => '\x0B',
                        Some(c) => {
                            self.str_builder.append_ascii(b'\\');
                            self.str_builder.append_char(c);
                            has_complex_escapes = true;
                            continue;
                        },
                        None => break,
                    };
                    self.str_builder.append_char(simple_escape);
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
            let error = CLexerError::UnendedString(location.clone());
            self.tokens.add_error_token(location, error);
        }

        self.add_token(CTokenKind::String {
            str_data: Arc::new(self.str_builder.current_as_box()),
            str_type,
            has_complex_escapes,
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
        self.add_token(CTokenKind::Number(num_data));
    }

    fn lex_identifier(&mut self, first_char: char) {
        let cached = self.read_cached_identifier(first_char);

        if let Some(keyword) = self.env.cached_to_keywords().get(&cached) {
            return self.add_token(CTokenKind::Keyword(*keyword, cached.uniq_id()));
        }

        if let Some(str_type) = self.env.cached_to_str_prefix().get(&cached).cloned() {
            let front_char = self.reader.front().unwrap_or('\0');
            if front_char == '"' || front_char == '\'' {
                return self.lex_string(str_type, front_char == '\'');
            }
        }

        self.add_token(CTokenKind::Identifier(cached));
    }

    fn lex_comment(&mut self, multi_line: bool) {
        loop {
            let char = match self.reader.move_forward() {
                Some(cl) => cl,
                None => {
                    if multi_line {
                        let location = self.reader.location();
                        let error = CLexerError::UnendedComment(location.clone());
                        self.tokens.add_error_token(location, error);
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
            self.tokens.append(CToken::new(
                self.reader.location(),
                false,
                CTokenKind::PreEnd,
            ));
        }
        self.at_start_of_line = true;
        self.have_skipped_whitespace = true;
        self.reader.move_forward();
    }

    fn add_token(&mut self, kind: CTokenKind) {
        let location = self.source_location();
        let token = CToken::new(location, self.have_skipped_whitespace, kind);
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

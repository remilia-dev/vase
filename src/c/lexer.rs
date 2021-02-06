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
        CError,
        CTokenStack,
        FileId,
    },
    sync::Arc,
    util::{
        CachedString,
        StringBuilder,
    },
};

#[derive(PartialEq)]
#[repr(u8)]
enum CLexerMode {
    Normal,
    Preprocessor,
    Include { next: bool },
    Message,
}

pub type CIncludeCallback<'a> =
    &'a (dyn Send + Sync + Fn(CIncludeType, &CachedString, &Option<Arc<Path>>) -> Option<FileId>);

pub struct CLexer<'a> {
    mode: CLexerMode,
    at_start_of_line: bool,
    env: &'a CCompileEnv,
    include_callback: CIncludeCallback<'a>,
    reader: CFileReader,
    loaded_path: Option<Arc<Path>>,
    str_builder: StringBuilder,
    norm_buffer: StringBuilder,
    link_stack: Vec<usize>,
}
impl<'a> CLexer<'a> {
    pub fn new(env: &'a CCompileEnv, include_callback: CIncludeCallback<'a>) -> CLexer<'a> {
        CLexer {
            mode: CLexerMode::Normal,
            at_start_of_line: true,
            env,
            include_callback,
            reader: CFileReader::new(),
            loaded_path: None,
            str_builder: StringBuilder::new(),
            norm_buffer: StringBuilder::new(),
            link_stack: Vec::with_capacity(5),
        }
    }

    pub fn lex_file(
        &mut self,
        file_id: FileId,
        file_path: Arc<Path>,
    ) -> Result<CTokenStack, CError> {
        // The scope is here to free file resources early.
        {
            let file = match File::open(&file_path) {
                Err(err) => return Err(CError::IOError(Arc::new(err))),
                Ok(f) => f,
            };

            if file.metadata().unwrap().len() == 0 {
                // Can't memory map a 0-byte file.
                let mut stack = CTokenStack::new(file_id, &Some(file_path));
                stack.append(CToken::new(0, 0, CTokenKind::EOF, false));
                return Result::Ok(stack);
            }

            // OPTIMIZATION: Would getting away from memory mapping be faster?
            // TODO: Lock the file that is being mapped. This would prevent the memory map from changing under us.
            // It would also allow this to be truly safe.
            let mmap = match unsafe { memmap2::MmapOptions::new().map(&file) } {
                Err(err) => return Err(CError::IOError(Arc::new(err))),
                Ok(m) => m,
            };

            if let Some(err) = self.reader.load_bytes(&mmap) {
                return Err(CError::Utf8DecodeError(err));
            }
        }

        self.loaded_path = Some(file_path);
        Result::Ok(self.lex(file_id))
    }

    pub fn lex_bytes(&mut self, file_id: FileId, bytes: &[u8]) -> Result<CTokenStack, CError> {
        if let Some(err) = self.reader.load_bytes(bytes) {
            return Result::Err(CError::Utf8DecodeError(err));
        }
        self.loaded_path = None;
        Result::Ok(self.lex(file_id))
    }

    #[must_use]
    fn lex(&mut self, file_id: FileId) -> CTokenStack {
        self.at_start_of_line = true;
        self.mode = CLexerMode::Normal;
        self.str_builder.clear();

        let mut tokens = CTokenStack::new(file_id, &self.loaded_path);
        if self.reader.is_empty() {
            tokens.append(CToken::new(0, 0, CTokenKind::EOF, false));
            tokens.finalize();
            return tokens;
        }

        let mut have_skipped_whitespace = false;
        loop {
            have_skipped_whitespace |= self.reader.skip_most_whitespace();

            let (character, position) = match self.reader.front() {
                CharResult::EOF => {
                    self.end_line(&mut tokens);
                    break;
                },
                CharResult::Value(value, position) => (value, position),
            };

            let kind = match character {
                '/' if self.reader.move_forward_if_next('/') => {
                    self.lex_comment(false);
                    continue;
                },
                '/' if self.reader.move_forward_if_next('*') => {
                    self.lex_comment(true);
                    have_skipped_whitespace = true;
                    continue;
                },
                '\n' => {
                    self.end_line(&mut tokens);
                    self.at_start_of_line = true;
                    have_skipped_whitespace = true;
                    continue;
                },
                '"' | '<' if matches!(self.mode, CLexerMode::Include { .. }) => {
                    self.lex_include(&mut tokens, character)
                },
                '\'' | '"' => self.lex_string(CStringType::Default, character),
                c if matches!(self.mode, CLexerMode::Message) => self.lex_message(c),
                c if r"~!@#%^&*()[]{}-+=:;\|,.<>/?".contains(c) => self.lex_symbol(&mut tokens, c),
                c if c.is_ascii_digit() => self.lex_number(false, c),
                c => self.lex_identifier(c),
            };

            let length = self.reader.distance_from(position);

            tokens.append(CToken::new(
                position,
                length,
                kind,
                have_skipped_whitespace,
            ));
            self.at_start_of_line = false;
            have_skipped_whitespace = false;
        }

        tokens.append(CToken::new(
            self.reader.last_byte(),
            0,
            CTokenKind::EOF,
            false,
        ));

        tokens.finalize();
        tokens
    }

    fn end_line(&mut self, tokens: &mut CTokenStack) {
        if self.mode != CLexerMode::Normal {
            self.mode = CLexerMode::Normal;
            tokens.append(CToken::new(
                self.reader.position(),
                0,
                CTokenKind::PreEnd,
                false,
            ));
        }
        self.reader.move_forward();
    }

    fn lex_symbol(&mut self, tokens: &mut CTokenStack, first_char: char) -> CTokenKind {
        let kind = match first_char {
            // TODO: Add double [[ and ]] support for C2X attributes
            '[' => CTokenKind::LBracket { alt: false },
            ']' => CTokenKind::RBracket { alt: false },
            '(' => CTokenKind::LParen,
            ')' => CTokenKind::RParen,
            '{' => CTokenKind::LBrace { alt: false },
            '}' => CTokenKind::RBrace { alt: false },
            '.' => {
                return match self.reader.move_forward().value_or_null_char() {
                    // This whole section returns early to allow parsing ... with moving backwards.
                    '.' => {
                        if self.reader.move_forward_if_next('.') {
                            self.reader.move_forward();
                            CTokenKind::DotDotDot
                        } else {
                            CTokenKind::Dot
                        }
                    },
                    c if c.is_ascii_digit() => return self.lex_number(true, c),
                    _ => CTokenKind::Dot,
                };
            },
            '&' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::AmpEqual,
                '&' => CTokenKind::AmpAmp,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Amp,
            },
            '*' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::StarEqual,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Star,
            },
            '+' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::PlusEqual,
                '+' => CTokenKind::PlusPlus,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Plus,
            },
            '-' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::MinusEqual,
                '-' => CTokenKind::MinusMinus,
                '>' => CTokenKind::Arrow,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Minus,
            },
            '~' => CTokenKind::Tilde,
            '!' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::BangEqual,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Bang,
            },
            '/' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::SlashEqual,
                // NOTE: Comments should have been handled in the main match in self.lex
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Slash,
            },
            '%' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::PercentEqual,
                '>' => CTokenKind::RBrace { alt: true },
                ':' => {
                    if self.reader.move_forward().value_or_null_char() == '%'
                        && self.reader.move_forward_if_next(':')
                    {
                        // Move past the last : (in %:%:)
                        self.reader.move_forward();
                        return CTokenKind::HashHash { alt: true };
                    }
                    // To prevent an extra move_forward, we return early.
                    return self.lex_preprocessor(tokens, true);
                },
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Percent,
            },
            '<' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::LAngleEqual,
                '<' => {
                    if self.reader.move_forward_if_next('=') {
                        CTokenKind::LShiftEqual
                    } else {
                        CTokenKind::LShift
                    }
                },
                '%' => CTokenKind::LBrace { alt: true },
                ':' => CTokenKind::LBracket { alt: true },
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::LAngle,
            },
            '>' => {
                match self.reader.move_forward().value_or_null_char() {
                    '>' => {
                        if self.reader.move_forward_if_next('=') {
                            CTokenKind::RShiftEqual
                        } else {
                            CTokenKind::RShift
                        }
                    },
                    '=' => CTokenKind::RAngleEqual,
                    // To prevent an extra move_forward, we return early.
                    _ => return CTokenKind::RAngle,
                }
            },
            '=' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::EqualEqual,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Equal,
            },
            '^' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::CarrotEqual,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Carrot,
            },
            '|' => match self.reader.move_forward().value_or_null_char() {
                '=' => CTokenKind::BarEqual,
                '|' => CTokenKind::BarBar,
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Bar,
            },
            '?' => CTokenKind::QMark,
            ':' => match self.reader.move_forward().value_or_null_char() {
                '>' => CTokenKind::RBracket { alt: true },
                // To prevent an extra move_forward, we return early.
                _ => return CTokenKind::Colon,
            },
            ';' => CTokenKind::Semicolon,
            ',' => CTokenKind::Comma,
            '#' => match self.reader.move_forward().value_or_null_char() {
                '#' => CTokenKind::HashHash { alt: false },
                // To prevent an extra move_forward, we return early.
                _ => return self.lex_preprocessor(tokens, false),
            },
            '@' => CTokenKind::At,
            '\\' => CTokenKind::Backslash,
            _ => unreachable!(
                "c_lex_symbol should never lex starting on the character {}.",
                first_char
            ),
        };

        // There should only be one symbol (for this token) in the reader that remains to be moved past.
        self.reader.move_forward();
        kind
    }

    fn lex_preprocessor(&mut self, tokens: &mut CTokenStack, alt_start: bool) -> CTokenKind {
        if self.mode == CLexerMode::Preprocessor || !self.at_start_of_line {
            return CTokenKind::Hash { alt: alt_start };
        }

        // The # or %: should have already been passed
        self.reader.skip_most_whitespace();

        let first_char = match self.reader.front() {
            CharResult::Value(val, ..) if val != '\n' => val,
            // If an error occurred, the EOF has been reached, or the end-of-line has been reached
            // we want to return a blank preprocessor instruction.
            _ => return CTokenKind::PreBlank,
        };

        let pre_id = self.read_cached_identifier(first_char);
        let pre_type = match self.env.cached_to_preprocessor().get(&pre_id) {
            Some(pre_type) => pre_type.clone(),
            None => {
                self.mode = CLexerMode::Preprocessor;
                return CTokenKind::PreUnknown(pre_id);
            },
        };

        if pre_type.ends_a_link() {
            let curr_index = tokens.len();
            match self.link_stack.pop() {
                Some(index) => tokens[index].kind_mut().set_link(curr_index),
                None => {
                    // TODO: Error about not-properly ended linking preprocessor
                    println!("TODO: Warn about not-properly ended linking preprocessor");
                },
            }
        }
        if pre_type.is_linking() {
            self.link_stack.push(tokens.len());
        }

        self.mode = match pre_type {
            CTokenKind::PreInclude => CLexerMode::Include { next: false },
            CTokenKind::PreIncludeNext => CLexerMode::Include { next: true },
            CTokenKind::PreError | CTokenKind::PreWarning => CLexerMode::Message,
            _ => CLexerMode::Preprocessor,
        };

        pre_type
    }

    fn lex_include(&mut self, tokens: &mut CTokenStack, include_start: char) -> CTokenKind {
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
        while let CharResult::Value(char, ..) = self.reader.move_forward() {
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
            // TODO: Communicate the warning
            println!("TODO: Include path was not properly ended.");
        }

        if let CLexerMode::Include { next } = self.mode {
            if next {
                inc_type = CIncludeType::IncludeNext;
            }
        }
        let path = self.env.cache().get_or_cache(self.str_builder.current());

        let inc_id = (self.include_callback)(inc_type, &path, &self.loaded_path);
        tokens.add_reference(&path, inc_id);

        CTokenKind::IncludePath { inc_type, path }
    }

    fn lex_message(&mut self, first_char: char) -> CTokenKind {
        self.str_builder.clear();
        self.str_builder.append_char(first_char);
        while let CharResult::Value(char, ..) = self.reader.move_forward() {
            if char == '\n' {
                break;
            }
            self.str_builder.append_char(char)
        }

        self.mode = CLexerMode::Normal;
        CTokenKind::Message(Arc::new(self.str_builder.current_as_box()))
    }

    fn lex_string(&mut self, str_type: CStringType, opening_char: char) -> CTokenKind {
        let mut ended_correctly = false;
        let mut has_complex_escapes = false;
        while let CharResult::Value(char, ..) = self.reader.move_forward() {
            match char {
                '\\' => {
                    let simple_escape = match self.reader.move_forward().value_or_null_char() {
                        '\'' => '\'',
                        '"' => '"',
                        '?' => '?',
                        '\\' => '\\',
                        'a' => '\x07',
                        'b' => '\x08',
                        'f' => '\x0C',
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'v' => '\x0B',
                        c => {
                            self.str_builder.append_ascii(b'\\');
                            self.str_builder.append_char(c);
                            has_complex_escapes = true;
                            continue;
                        },
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
            // TODO: Communicate the warning
            println!("TODO: Missing end character for string.");
        }

        CTokenKind::String {
            str_data: Arc::new(self.str_builder.current_as_box()),
            str_type,
            has_complex_escapes,
            is_char: opening_char == '\'',
        }
    }

    fn lex_number(&mut self, dot_start: bool, first_char: char) -> CTokenKind {
        self.str_builder.clear();
        if dot_start {
            self.str_builder.append_ascii(b'.');
        }

        let mut num_type = CNumberType::Dec;
        if !dot_start && first_char == '0' {
            num_type = if self.reader.move_forward_if_next('x') {
                CNumberType::Hex
            } else if self.reader.move_forward_if_next('b') {
                CNumberType::Bin
            } else {
                CNumberType::Oct
            }
        } else {
            // NOTE: All characters in a number are ascii
            self.str_builder.append_ascii(first_char as u8);
        }

        while let CharResult::Value(char, ..) = self.reader.move_forward() {
            match char {
                c if num_type.supports_exp(c) => {
                    // NOTE: All supported exponents start with an ascii character.
                    self.str_builder.append_ascii(c as u8);
                    if self.reader.move_forward_if_next('-') {
                        self.str_builder.append_ascii(b'-');
                    } else if self.reader.move_forward_if_next('+') {
                        // We don't need the + to properly parse it, so we discard it.
                    }
                },
                '.' | '_' => self.str_builder.append_ascii(char as u8),
                c if c.is_whitespace() | c.is_ascii_punctuation() => break,
                c => self.str_builder.append_char(c),
            }
        }

        let num_data = self.env.cache().get_or_cache(self.str_builder.current());
        CTokenKind::Number { num_type, num_data }
    }

    fn lex_identifier(&mut self, first_char: char) -> CTokenKind {
        let cached = self.read_cached_identifier(first_char);

        if let Some(keyword) = self.env.cached_to_keywords().get(&cached) {
            return keyword.clone();
        }

        if let Some(str_type) = self.env.cached_to_str_prefix().get(&cached).cloned() {
            let front_char = self.reader.front().value_or_null_char();
            if front_char == '"' || front_char == '\'' {
                return self.lex_string(str_type, front_char);
            }
        }

        CTokenKind::Identifier(cached)
    }

    fn read_cached_identifier(&mut self, first_char: char) -> CachedString {
        self.str_builder.clear();
        self.str_builder.append_char(first_char);

        while let CharResult::Value(char, ..) = self.reader.move_forward() {
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

    fn lex_comment(&mut self, multi_line: bool) {
        loop {
            let char = match self.reader.move_forward() {
                CharResult::Value(cv, ..) => cv,
                CharResult::EOF => {
                    if multi_line {
                        // TODO: Communicate the warning
                        println!("TODO: End-of-file hit before the end of multi-line comment.");
                    }
                    return;
                },
            };

            match char {
                '\n' if !multi_line => return,
                '*' if multi_line => {
                    if self.reader.move_forward_if_next('/') {
                        self.reader.move_forward();
                        return;
                    }
                },
                _ => {},
            }
        }
    }
}

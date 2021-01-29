// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::util::{
    CharExt,
    SourceLocation,
    Utf8DecodeError,
};

pub struct CharReader {
    line_chars: Vec<CharPos>,
    position: usize,
}

impl CharReader {
    pub fn new() -> Self {
        CharReader {
            line_chars: Vec::with_capacity(1000),
            position: 0,
        }
    }

    #[must_use]
    pub fn load_bytes(&mut self, bytes: &[u8]) -> Option<Utf8DecodeError> {
        self.position = 0;
        self.line_chars.clear();

        let mut byte_pos = 0usize;
        let mut char_pos = 0u32;
        let mut column = 0u32;
        let mut line = 0u32;
        while byte_pos < bytes.len() {
            let char_bytes = match char::decode_utf8(bytes, byte_pos) {
                Ok(cb) => cb,
                Err(err) => {
                    self.line_chars.clear();
                    return Option::Some(err);
                },
            };

            byte_pos += char_bytes.byte_count();

            let add_char = match char_bytes.char() {
                '\\' if byte_pos < bytes.len() => match bytes[byte_pos] {
                    b'\r' if byte_pos + 1 < bytes.len() && bytes[byte_pos + 1] == b'\n' => {
                        column = 0;
                        line += 1;
                        byte_pos += 2;
                        char_pos += 3;
                        continue;
                    },
                    b'\n' => {
                        column = 0;
                        line += 1;
                        byte_pos += 1;
                        char_pos += 2;
                        continue;
                    },
                    _ => '\\',
                },
                '\n' => {
                    self.line_chars
                        .push(CharPos { char: '\n', line, column, pos: char_pos });
                    column = 0;
                    line += 1;
                    char_pos += 1;
                    continue;
                },
                // TODO: Trigraph support could be added here (remember ??/ acts like \ )
                c => c,
            };

            self.line_chars.push(CharPos {
                char: add_char,
                line,
                column,
                pos: char_pos,
            });
            column += 1;
            char_pos += 1;
        }

        Option::None
    }

    pub fn is_empty(&self) -> bool {
        self.line_chars.is_empty()
    }

    pub fn front(&self) -> CharResult {
        if self.position >= self.line_chars.len() {
            CharResult::EOF
        } else {
            let char_pos = &self.line_chars[self.position];
            CharResult::Value(char_pos.char, char_pos.pos)
        }
    }

    pub fn move_forward(&mut self) -> CharResult {
        self.position += 1;
        self.front()
    }

    pub fn position(&self) -> SourceLocation {
        let char_pos = &self.line_chars[self.position.min(self.line_chars.len() - 1)];
        SourceLocation::new(char_pos.line, char_pos.column)
    }

    pub fn next_char_or_null(&self) -> char {
        let next_position = self.position + 1;
        if next_position >= self.line_chars.len() {
            '\0'
        } else {
            self.line_chars[next_position].char
        }
    }

    pub fn move_forward_if_next(&mut self, c: char) -> bool {
        if self.next_char_or_null() == c {
            self.move_forward();
            return true;
        }
        false
    }

    pub fn skip_most_whitespace(&mut self) -> bool {
        let front = self.front().value_or_null_char();
        if !front.is_whitespace() || front == '\n' {
            return false;
        }

        while let CharResult::Value(front, ..) = self.move_forward() {
            if !front.is_whitespace() || front == '\n' {
                break;
            }
        }
        true
    }

    pub fn char_distance_from(&self, position: u32) -> u32 {
        return if self.position >= self.line_chars.len() {
            self.line_chars.last().unwrap().pos + 1 - position
        } else {
            self.line_chars[self.position].pos - position
        };
    }
}

impl Default for CharReader {
    fn default() -> Self {
        CharReader::new()
    }
}

pub enum CharResult {
    Value(char, u32),
    EOF,
}

impl CharResult {
    pub fn value_or_null_char(&self) -> char {
        match self {
            CharResult::Value(v, ..) => *v,
            _ => '\0',
        }
    }
}

struct CharPos {
    char: char,
    line: u32,
    column: u32,
    pos: u32,
}

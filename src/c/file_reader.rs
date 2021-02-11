// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::util::{
    CharExt,
    Utf8DecodeError,
};

pub struct CFileReader {
    line_chars: Vec<(char, u32)>,
    position: usize,
    last_byte: u32,
}

impl CFileReader {
    pub fn new() -> Self {
        CFileReader {
            line_chars: Vec::with_capacity(1000),
            position: 0,
            last_byte: 0,
        }
    }

    #[must_use]
    pub fn load_bytes(&mut self, bytes: &[u8]) -> Option<Utf8DecodeError> {
        self.position = 0;
        self.line_chars.clear();

        let mut byte_pos = 0usize;
        while byte_pos < bytes.len() {
            let char_bytes = match char::decode_utf8(bytes, byte_pos) {
                Ok(cb) => cb,
                Err(err) => {
                    self.line_chars.clear();
                    return Option::Some(err);
                },
            };

            let add_char = match char_bytes.char() {
                '\\' => match bytes.get(byte_pos + 1) {
                    Some(b'\r') if bytes.get(byte_pos + 2) == Some(&b'\n') => {
                        byte_pos += 3;
                        continue;
                    },
                    Some(b'\n') => {
                        byte_pos += 2;
                        continue;
                    },
                    _ => '\\',
                },
                // TODO: Trigraph support could be added here (remember ??/ acts like \ )
                c => c,
            };

            self.line_chars.push((add_char, byte_pos as u32));

            byte_pos += char_bytes.byte_count();
        }

        self.last_byte = byte_pos as u32;

        Option::None
    }

    pub fn last_byte(&self) -> u32 {
        self.last_byte
    }

    pub fn is_empty(&self) -> bool {
        self.line_chars.is_empty()
    }

    pub fn front(&self) -> CharResult {
        if self.position >= self.line_chars.len() {
            CharResult::Eof
        } else {
            let (char, pos) = &self.line_chars[self.position];
            CharResult::Value(*char, *pos)
        }
    }

    pub fn move_forward(&mut self) -> CharResult {
        self.position += 1;
        self.front()
    }

    pub fn next_char_or_null(&self) -> char {
        let next_position = self.position + 1;
        if next_position >= self.line_chars.len() {
            '\0'
        } else {
            self.line_chars[next_position].0
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

    pub fn position(&self) -> u32 {
        if self.position < self.line_chars.len() {
            self.line_chars[self.position].1
        } else {
            self.last_byte
        }
    }

    pub fn distance_from(&self, position: u32) -> u32 {
        if self.position < self.line_chars.len() {
            self.line_chars[self.position].1 - position
        } else {
            self.line_chars.last().unwrap().1 + 1 - position
        }
    }
}

impl Default for CFileReader {
    fn default() -> Self {
        CFileReader::new()
    }
}

pub enum CharResult {
    Value(char, u32),
    Eof,
}

impl CharResult {
    pub fn value_or_null_char(&self) -> char {
        match self {
            CharResult::Value(v, ..) => *v,
            _ => '\0',
        }
    }
}

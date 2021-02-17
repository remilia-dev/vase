// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryFrom;

use crate::util::{
    CharExt,
    Utf8DecodeError,
};

pub struct CFileReader {
    line_chars: Vec<CharLocation>,
    position: usize,
    last_byte: u32,
    length_accum: u32,
}

impl CFileReader {
    pub fn new() -> Self {
        CFileReader {
            line_chars: Vec::with_capacity(1000),
            position: 0,
            last_byte: 0,
            length_accum: 0,
        }
    }

    #[must_use]
    pub fn load_bytes(&mut self, bytes: &[u8]) -> Option<Utf8DecodeError> {
        self.position = 0;
        self.line_chars.clear();

        let mut byte_pos = 0usize;
        let mut char_length = 0u32;
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
                        char_length += 3;
                        continue;
                    },
                    Some(b'\n') => {
                        byte_pos += 2;
                        char_length += 2;
                        continue;
                    },
                    _ => '\\',
                },
                // OPTIMIZATION: Skip all spaces after a new line character (they can't be within strings)
                // TODO: Trigraph support could be added here (remember ??/ acts like \ )
                c => c,
            };

            self.line_chars.push(CharLocation {
                char: add_char,
                byte: u32::try_from(byte_pos).unwrap_or(u32::MAX),
                length: char_length + char_bytes.byte_count() as u32,
            });

            byte_pos += char_bytes.byte_count();
            char_length = 0;
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

    pub fn front(&self) -> Option<char> {
        self.front_location().map(|cl| cl.char)
    }

    pub fn front_location(&self) -> Option<CharLocation> {
        self.line_chars.get(self.position).cloned()
    }

    pub fn move_forward(&mut self) -> Option<char> {
        self.length_accum += self.front_location().map_or(0, |cl| {
            // If the accumulator is at 0, we use the UTF-8 length to avoid including escape-newlines.
            // How this behavior is implemented would have to be done differently to support trigraphs.
            if self.length_accum == 0 {
                cl.char.len_utf8() as u32
            } else {
                cl.length
            }
        });
        self.position += 1;
        self.front()
    }

    pub fn next_char(&self) -> Option<char> {
        self.line_chars.get(self.position + 1).map(|c| c.char)
    }

    pub fn move_forward_if_next(&mut self, c: char) -> bool {
        if self.next_char() == Some(c) {
            self.move_forward();
            return true;
        }
        false
    }

    pub fn skip_most_whitespace(&mut self) -> bool {
        match self.front() {
            // New lines are handled by the lexer in some scenarios, so we can't skip them.
            Some('\n') => return false,
            Some(c) if c.is_whitespace() => {},
            _ => return false,
        }

        loop {
            self.position += 1;
            match self.front() {
                Some('\n') => break,
                Some(c) if c.is_whitespace() => {},
                _ => break,
            }
        }

        self.length_accum = 0;
        true
    }

    pub fn position(&self) -> u32 {
        self.line_chars
            .get(self.position)
            .map(|c| c.byte)
            .unwrap_or(self.last_byte)
    }

    pub fn get_and_clear_length(&mut self) -> u32 {
        let length = self.length_accum;
        self.length_accum = 0;
        length
    }
}

impl Default for CFileReader {
    fn default() -> Self {
        CFileReader::new()
    }
}

#[derive(Copy, Clone)]
pub struct CharLocation {
    char: char,
    byte: u32,
    length: u32,
}

impl CharLocation {
    pub fn char(&self) -> char {
        self.char
    }
    pub fn byte(&self) -> u32 {
        self.byte
    }
}

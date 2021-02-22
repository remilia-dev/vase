// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryFrom;

use crate::util::{
    CharExt,
    FileId,
    SourceLocation,
    Utf8DecodeError,
};

pub struct FileReader {
    line_chars: Vec<CharLocation>,
    position: usize,
    file_id: FileId,
    last_byte: u32,
}

impl FileReader {
    pub fn new() -> Self {
        FileReader {
            line_chars: Vec::with_capacity(1000),
            position: 0,
            file_id: FileId::MAX,
            last_byte: 0,
        }
    }

    #[must_use]
    pub fn load_bytes(&mut self, file_id: FileId, bytes: &[u8]) -> Option<Utf8DecodeError> {
        self.position = 0;
        self.file_id = file_id;
        self.line_chars.clear();

        let mut byte_pos = 0usize;
        while byte_pos < bytes.len() {
            let char_bytes = match char::decode_utf8(bytes, byte_pos) {
                Ok(cb) => cb,
                Err(err) => {
                    self.line_chars.clear();
                    return Some(err);
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
                // OPTIMIZATION: Skip all spaces after a new line character (they can't be within strings)
                // TODO: Trigraph support could be added here (remember ??/ acts like \ )
                c => c,
            };

            self.line_chars.push(CharLocation {
                char: add_char,
                byte: u32::try_from(byte_pos).unwrap_or(u32::MAX),
                length: char_bytes.byte_count() as u32,
            });

            byte_pos += char_bytes.byte_count();
        }

        self.last_byte = byte_pos as u32;

        None
    }

    pub fn last_byte(&self) -> u32 {
        self.last_byte
    }

    pub fn is_empty(&self) -> bool {
        self.line_chars.is_empty()
    }

    pub fn front(&self) -> Option<char> {
        self.line_chars.get(self.position).map(|cl| cl.char)
    }

    pub fn front_location(&self) -> Option<(char, SourceLocation)> {
        let cl = self.line_chars.get(self.position)?;
        let location = SourceLocation::new(self.file_id, cl.byte, cl.length as u16);
        Some((cl.char, location))
    }

    pub fn location(&self) -> SourceLocation {
        if let Some(cl) = self.line_chars.get(self.position) {
            SourceLocation::new(self.file_id, cl.byte, cl.length as u16)
        } else {
            SourceLocation::new(self.file_id, self.last_byte, 0)
        }
    }

    pub fn previous_location(&self) -> SourceLocation {
        if let Some(cl) = self.line_chars.get(self.position - 1) {
            SourceLocation::new(self.file_id, cl.byte, cl.length as u16)
        } else {
            SourceLocation::new(self.file_id, self.last_byte, 0)
        }
    }

    pub fn move_forward(&mut self) -> Option<char> {
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
        true
    }

    pub fn position(&self) -> u32 {
        self.line_chars
            .get(self.position)
            .map(|c| c.byte)
            .unwrap_or(self.last_byte)
    }
}

impl Default for FileReader {
    fn default() -> Self {
        FileReader::new()
    }
}

#[derive(Copy, Clone)]
struct CharLocation {
    char: char,
    byte: u32,
    length: u32,
}

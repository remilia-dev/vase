// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

/// A trait that adds some extension methods to [char].
pub trait CharExt: private::Sealed {
    /// Returns true if the a char value is within the character range `'0'..='7'`.
    fn is_ascii_octdigit(&self) -> bool;
    /// Converts a hexdigit (see [char::is_ascii_hexdigit]) to a byte value.
    /// Only up to the first 4 bits will be set.
    /// # Panics
    /// Panics when char::is_ascii_hexdigit is false.
    fn hexdigit_as_byte(&self) -> u8;
    /// Attempts to decode a UTF-8 character from an array of bytes at a given offset.
    ///
    /// Should decoding fail, an error describing the problem will be returned.
    fn decode_utf8(bytes: &[u8], index: usize) -> Result<DecodedChar, Utf8DecodeError>;
}

impl CharExt for char {
    fn is_ascii_octdigit(&self) -> bool {
        '0' <= *self && *self <= '7'
    }

    fn hexdigit_as_byte(&self) -> u8 {
        match self {
            '0'..='9' => *self as u8 - b'0',
            'a'..='f' => (*self as u8 - b'a') + 10,
            'A'..='F' => (*self as u8 - b'A') + 10,
            _ => panic!("Non-hexadecimal character passed to hexdigit_as_byte."),
        }
    }

    fn decode_utf8(bytes: &[u8], offset: usize) -> Result<DecodedChar, Utf8DecodeError> {
        let first_byte = bytes[offset];
        let byte_count = first_byte.leading_ones() as usize;

        match byte_count {
            0 => {
                // SAFETY: Bytes with no leading 1s are ASCII characters.
                let char = unsafe { std::char::from_u32_unchecked(first_byte as u32) };

                return Ok(DecodedChar { char, byte_count: 1 });
            },
            1 => return Err(Utf8DecodeError::MisalignedRead { byte_position: offset }),
            2..=4 => {
                if offset + byte_count > bytes.len() {
                    return Err(Utf8DecodeError::MissingBytes {
                        byte_position: offset,
                        missing_byte_count: offset + byte_count - bytes.len(),
                    });
                }
            },
            _ => {
                return Err(Utf8DecodeError::InvalidByte {
                    byte_position: offset,
                    bad_byte: first_byte,
                });
            },
        }

        let raw_char: u32;
        let mask_check: u32;
        match byte_count {
            2 => {
                raw_char = (first_byte as u32 & 0b0001_1111u32) << 6
                    | (bytes[offset + 1] as u32 & 0b0011_1111u32);
                // One of these three bits must be set otherwise it's an over-long encoding (which is invalid).
                mask_check = 0b111_1000_0000;
            },
            3 => {
                raw_char = (first_byte as u32 & 0b0000_1111u32) << 12
                    | (bytes[offset + 1] as u32 & 0b0011_1111u32) << 6
                    | (bytes[offset + 2] as u32 & 0b0011_1111u32);
                // One of these five bits must be set otherwise it's an over-long encoding (which is invalid).
                mask_check = 0b1111_1000_0000_0000;
            },
            4 => {
                raw_char = (first_byte as u32 & 0b0000_0111u32) << 18
                    | (bytes[offset + 1] as u32 & 0b0011_1111u32) << 12
                    | (bytes[offset + 2] as u32 & 0b0011_1111u32) << 6
                    | (bytes[offset + 3] as u32 & 0b0011_1111u32);
                // One of these five bits must be set otherwise it's an over-long encoding (which is invalid).
                mask_check = 0b1_1111_0000_0000_0000_0000
            },
            _ => unreachable!(),
        };

        let char_val = match std::char::from_u32(raw_char) {
            Some(char) => char,
            None => {
                return Err(Utf8DecodeError::InvalidCharacter {
                    byte_position: offset,
                    bad_codepoint: raw_char,
                });
            },
        };

        if (raw_char & mask_check) == 0 {
            return Err(Utf8DecodeError::OverlongEncoding {
                byte_position: offset,
                bad_byte_count: byte_count,
                encoded_character: char_val,
            });
        }

        Ok(DecodedChar { char: char_val, byte_count })
    }
}

/// An error that details a failed attempt at decoding a UTF-8 character.
/// See [CharExt::decode_utf8]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Utf8DecodeError {
    /// Decoding failed because it started on a byte that should never occur in UTF-8.
    /// This means the byte started with 5 or more 1s (`248..=255`).
    InvalidByte {
        /// The byte offset that the byte occurs at.
        byte_position: usize,
        /// The bad byte that isn't valid in UTF-8.
        bad_byte: u8,
    },
    /// Decoding resulted in an invalid character. Essentially [std::char::from_u32] returned None.
    InvalidCharacter {
        /// The byte offset of the start of the character.
        byte_position: usize,
        /// The codepoint that was decoded but invalid.
        bad_codepoint: u32,
    },
    /// Decoding failed because the character was encoded with more bytes than required
    /// (which is not valid UTF-8). For example, an ASCII character encoded with 2 bytes
    /// (it only requires 1) would be an overlong encoding.
    ///
    /// It's likely the bytes aren't actually UTF-8.
    OverlongEncoding {
        /// The byte offset of the start of the character.
        byte_position: usize,
        /// The number of bytes the character was encoded with.
        bad_byte_count: usize,
        /// The character that was encoded.
        encoded_character: char,
    },
    /// Decoding failed because it started on a byte with a single leading 1.
    /// In UTF-8, these bytes should only exist after the first byte of a multi-byte character.
    ///
    /// Unless you decoded at an incorrect offset, it's likely the bytes are not UTF-8.
    MisalignedRead {
        /// The byte offset of the start of the character.
        byte_position: usize,
    },
    /// Decoding failed because the character requires more bytes than in the array.
    MissingBytes {
        /// The byte offset of the start of the character.
        byte_position: usize,
        /// The number of bytes missing from the end of the array.
        missing_byte_count: usize,
    },
}
impl std::fmt::Display for Utf8DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::InvalidByte { byte_position, bad_byte } => write!(
                f,
                "The byte at 0x{:X?} is an invalid value (0x{:X?}).",
                byte_position, bad_byte
            ),
            Self::InvalidCharacter { byte_position, bad_codepoint } => write!(
                f,
                "Read an invalid codepoint (0x{:X?}) at byte 0x{:X?}.",
                bad_codepoint, byte_position
            ),
            Self::OverlongEncoding {
                byte_position,
                bad_byte_count,
                encoded_character,
            } => write!(
                f,
                "At byte 0x{:X?}, a character ({}) was over-long encoded with {} bytes.",
                byte_position, encoded_character, bad_byte_count
            ),
            Self::MisalignedRead { byte_position } => write!(
                f,
                "A misaligned read occurred at byte 0x{:X?}.",
                byte_position
            ),
            Self::MissingBytes { byte_position, missing_byte_count } => write!(
                f,
                "A character starting at byte 0x{:X?} requires {} more byte(s) to decode.",
                byte_position, missing_byte_count
            ),
        }
    }
}
impl std::error::Error for Utf8DecodeError {}

/// A char and the number of bytes it took.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct DecodedChar {
    char: char,
    byte_count: usize,
}
impl DecodedChar {
    /// Returns the char that was decoded.
    pub fn char(&self) -> char {
        self.char
    }
    /// Returns the number of bytes that encoded this char (`1..=4`).
    pub fn byte_count(&self) -> usize {
        self.byte_count
    }
}

// This exists to prevent others from implemented CharExt.
mod private {
    pub trait Sealed {}
    impl Sealed for char {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_ascii_octdigit_is_true_for_octal_digits() {
        assert!('0'.is_ascii_octdigit());
        assert!('1'.is_ascii_octdigit());
        assert!('2'.is_ascii_octdigit());
        assert!('3'.is_ascii_octdigit());
        assert!('4'.is_ascii_octdigit());
        assert!('5'.is_ascii_octdigit());
        assert!('6'.is_ascii_octdigit());
        assert!('7'.is_ascii_octdigit());
    }

    #[test]
    fn is_ascii_octdigit_is_false_for_non_octal_digits() {
        assert!(!'8'.is_ascii_octdigit());
        assert!(!'9'.is_ascii_octdigit());
        assert!(!'a'.is_ascii_octdigit());
    }

    #[test]
    fn hexdigit_as_byte_returns_correct_value() {
        for (test, val) in ('0'..='9').zip(0..=9u8) {
            assert_eq!(test.hexdigit_as_byte(), val);
        }
        for (test, val) in ('a'..='f').zip(10..=15u8) {
            assert_eq!(test.hexdigit_as_byte(), val);
        }
        for (test, val) in ('A'..='F').zip(10..=15u8) {
            assert_eq!(test.hexdigit_as_byte(), val);
        }
    }

    #[test]
    #[should_panic]
    fn hexdigit_as_byte_panics_on_non_hex_digit() {
        'g'.hexdigit_as_byte();
    }

    #[test]
    fn decode_utf8_correctly_decodes_ascii_char() {
        let bytes = [b'a', b'B'];
        let decode1 = char::decode_utf8(&bytes, 0);
        assert_eq!(decode1.unwrap(), DecodedChar { char: 'a', byte_count: 1 });
        let decode2 = char::decode_utf8(&bytes, 1);
        assert_eq!(decode2.unwrap(), DecodedChar { char: 'B', byte_count: 1 });
    }

    #[test]
    fn decode_utf8_correctly_decodes_multibyte_chars() {
        let bytes = "¢€𐍈".as_bytes();
        let decode1 = char::decode_utf8(&bytes, 0);
        assert_eq!(decode1.unwrap(), DecodedChar {
            char: '¢', byte_count: 2
        });
        let decode2 = char::decode_utf8(&bytes, 2);
        assert_eq!(decode2.unwrap(), DecodedChar {
            char: '€', byte_count: 3
        });
        let decode3 = char::decode_utf8(&bytes, 5);
        assert_eq!(decode3.unwrap(), DecodedChar {
            char: '𐍈', byte_count: 4
        });
    }

    #[test]
    fn decode_utf8_returns_correct_error() {
        // A byte that starts with 10 should only occur in the middle of one UTF-8 character.
        let err1 = char::decode_utf8(&[0b1000_0000u8], 0);
        assert_eq!(err1.unwrap_err(), Utf8DecodeError::MisalignedRead {
            byte_position: 0
        });
        // A byte that starts with 110 requires there to be a 10-byte to follow it.
        let err2 = char::decode_utf8(&[0b1100_0000u8], 0);
        assert_eq!(err2.unwrap_err(), Utf8DecodeError::MissingBytes {
            byte_position: 0,
            missing_byte_count: 1
        });
        // A byte that starts with 1111_10 or 1111_110 or 1111_1110 or 0b1111_1111 is invalid.
        let err3 = char::decode_utf8(&[0b1111_1000], 0);
        assert_eq!(err3.unwrap_err(), Utf8DecodeError::InvalidByte {
            byte_position: 0,
            bad_byte: 0b1111_1000
        });
        let err4 = char::decode_utf8(&[0b1111_0111, 0b1011_1111, 0b1011_1111, 0b1011_1111], 0);
        assert_eq!(err4.unwrap_err(), Utf8DecodeError::InvalidCharacter {
            byte_position: 0,
            bad_codepoint: 0x1F_FFFF
        });
        // A UTF-8 character *must* use the minimum number of bytes possible.
        let err5 = char::decode_utf8(&[0b1100_0000, 0b1011_1111], 0);
        assert_eq!(err5.unwrap_err(), Utf8DecodeError::OverlongEncoding {
            byte_position: 0,
            bad_byte_count: 2,
            encoded_character: '?'
        });
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

use crate::math::{
    Integer,
    NumberResult,
    Real,
};

/// Represents a base a number can have.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NumBase {
    Binary,
    Octal,
    Decimal,
    Hexadecimal,
}

impl NumBase {
    /// Returns true if the given character is a digit for this base.
    /// ```
    /// # use vase::math::NumBase;
    /// assert!(NumBase::Octal.is_digit('3'));
    /// assert!(!NumBase::Binary.is_digit('3'));
    /// assert!(NumBase::Hexadecimal.is_digit('A'));
    /// ```
    pub fn is_digit(self, c: char) -> bool {
        if c.is_ascii() {
            self.is_digit_ascii(c as u8)
        } else {
            false
        }
    }
    /// Returns true if the given ASCII character is a digit for this base.
    /// ```
    /// # use vase::math::NumBase;
    /// assert!(NumBase::Octal.is_digit_ascii(b'3'));
    /// assert!(!NumBase::Binary.is_digit_ascii(b'3'));
    /// assert!(NumBase::Hexadecimal.is_digit_ascii(b'A'));
    /// ```
    pub fn is_digit_ascii(self, c: u8) -> bool {
        match c {
            b'0' | b'1' => true,
            b'2' | b'3' | b'4' | b'5' | b'6' | b'7' => self != NumBase::Binary,
            b'8' | b'9' => self >= NumBase::Decimal,
            b'a' | b'A' | b'b' | b'B' | b'c' | b'C' | b'd' | b'D' | b'e' | b'E' | b'f' | b'F' => {
                self == NumBase::Hexadecimal
            },
            _ => false,
        }
    }
    /// Converts the ASCII character to the digit's value.
    /// If this character is not a digit, None is returned.
    pub fn digit_to_value(self, c: u8) -> Option<u8> {
        match c {
            b'0' | b'1' => Some(c - b'0'),
            b'2'..=b'7' if self != NumBase::Binary => Some(c - b'0'),
            b'8'..=b'9' if self >= NumBase::Decimal => Some(c - b'0'),
            b'a'..=b'f' if self == NumBase::Hexadecimal => Some(c as u8 - b'a' + 10),
            b'A'..=b'F' if self == NumBase::Hexadecimal => Some(c as u8 - b'A' + 10),
            _ => None,
        }
    }
    /// Returns the 'radix' (base).
    /// ```
    /// # use vase::math::NumBase;
    /// assert_eq!(NumBase::Binary.radix(), 2);
    /// assert_eq!(NumBase::Decimal.radix(), 10);
    /// ```
    pub fn radix(self) -> u8 {
        match self {
            Self::Binary => 2,
            Self::Octal => 8,
            Self::Decimal => 10,
            Self::Hexadecimal => 16,
        }
    }
    /// Finds the index of the first byte that is not a valid digit
    /// and if a dot was passed.
    ///
    /// If `can_pass_dot` is true, the function will pass only 1 `b'.'`
    /// (a second dot will cause an error).
    /// If `can_past_dot` is false, the second value of the tuple will
    /// always be false.
    pub fn find_end_of_digits<T>(self, digits: T, mut can_pass_dot: bool) -> (usize, bool)
    where T: AsRef<[u8]> {
        let digits = digits.as_ref();
        let mut have_passed_dot = false;
        for (pos, c) in digits.iter().enumerate() {
            if *c == b'.' && can_pass_dot {
                have_passed_dot = true;
                can_pass_dot = false;
            } else if !self.is_digit_ascii(*c) {
                return (pos, have_passed_dot);
            }
        }
        (digits.len(), have_passed_dot)
    }
    /// Attempts to parse a numerical string (containing no dot).
    /// If the numerical string contains a non-digit character (for
    /// this base), an error will be returned.
    ///
    /// The string will be parsed at the precision of the integer
    /// type given. Should the numerical string be too large to fit
    /// in this type, wrapping-overflow will occur.
    pub fn parse_int<N, R>(self, number: R) -> NumberResult<N>
    where
        N: Integer,
        R: AsRef<[u8]>,
    {
        crate::math::parsing::parse_int(self, number.as_ref())
    }
    /// Attempts to parse a numerical string (potentially containing a dot).
    /// If the numerical string contains a non-digit character (for
    /// this base), an error will be returned.
    ///
    /// The function will be parsed at the precision of the real
    /// type given. 'Overflow' can only occur if there are so many digits
    /// that the real becomes infinite.
    /// # Rounding
    /// This function *should* round the numerical string to the closest
    /// possible value. However, **this has not been fully tested**.
    /// # Exponents
    /// This function does *not* handle exponent postfixes (like `1E10`).
    /// Postfixes should be handled separately.
    pub fn parse_real<N, R>(self, number: R) -> NumberResult<N>
    where
        N: Real,
        R: AsRef<[u8]>,
    {
        crate::math::parsing::parse_real(self, number.as_ref())
    }
}

impl fmt::Display for NumBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug-printing will print the name of the base.
        write!(f, "{:?}", self)
    }
}

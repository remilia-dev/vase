// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;
use std::ops::{
    AddAssign,
    DivAssign,
    MulAssign,
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
    /// # use vase::util::NumBase;
    /// assert!(NumBase::Octal.is_digit('3'));
    /// assert!(!NumBase::Binary.is_digit('3'));
    /// assert!(NumBase::Hexadecimal.is_digit('A'));
    /// ```
    pub fn is_digit(self, c: char) -> bool {
        match c {
            '0' | '1' => true,
            '2' | '3' | '4' | '5' | '6' | '7' => self != NumBase::Binary,
            '8' | '9' => self >= NumBase::Decimal,
            'a' | 'A' | 'b' | 'B' | 'c' | 'C' | 'd' | 'D' | 'e' | 'E' | 'f' | 'F' => {
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
    /// # use vase::util::NumBase;
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
}

/// Represents a type that [parse_int] can handle.
pub trait ParsableInt: Sized + From<u8> + Clone {
    /// Adds to the int and potentially overflows.
    /// If overflow occurs, true should be returned.
    /// When the value overflows, the value should be wrapped.
    fn overflowing_add(self, rhs: Self) -> (Self, bool);
    /// Multiplies to the int and potentially overflows.
    /// If overflow occurs, true should be returned.
    /// When the value overflows, the value should be wrapped.
    fn overflowing_mul(self, rhs: Self) -> (Self, bool);
}
impl ParsableInt for usize {
    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        usize::overflowing_add(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        usize::overflowing_mul(self, rhs)
    }
}
impl ParsableInt for u32 {
    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        u32::overflowing_add(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        u32::overflowing_mul(self, rhs)
    }
}
impl ParsableInt for u64 {
    fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        u64::overflowing_add(self, rhs)
    }

    fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        u64::overflowing_mul(self, rhs)
    }
}

/// Attempts to parse and return a numerical string of the given base.
/// If the numerical string contains non-digit characters, an error will be returned.
pub fn parse_int<T: ParsableInt>(base: NumBase, number: &str) -> NumberResult<T> {
    let radix = T::from(base.radix());
    let mut accum = T::from(0);
    let mut overflowed = false;
    for (i, c) in number.as_bytes().iter().enumerate() {
        if let Some(raw_digit) = base.digit_to_value(*c) {
            let digit = T::from(raw_digit);
            let (shifted, overflow) = accum.overflowing_mul(radix.clone());
            overflowed |= overflow;
            let (accumulated, overflow) = shifted.overflowing_add(digit);
            overflowed |= overflow;
            accum = accumulated;
        } else {
            return Err(ParseNumberError::NonDigitByte { byte: *c, index: i });
        }
    }

    Ok(ParsedNumber {
        number: accum,
        overflowed,
        excess_precision: 0,
    })
}

/// Represents a type that [parse_real] can handle.
pub trait ParsableReal:
    Sized + From<u8> + MulAssign + AddAssign + DivAssign + PartialOrd + Clone
{
    /// Raises the real to a specific power.
    fn pow(self, rhs: i32) -> Self;
    /// Checks whether the real is infinity or not.
    /// This is used to see if a float 'overflowed' into an infinity.
    fn is_infinity(&self) -> bool;
}
impl ParsableReal for f32 {
    fn pow(self, rhs: i32) -> Self {
        f32::powi(self, rhs)
    }

    fn is_infinity(&self) -> bool {
        f32::is_infinite(*self)
    }
}
impl ParsableReal for f64 {
    fn pow(self, rhs: i32) -> Self {
        f64::powi(self, rhs)
    }

    fn is_infinity(&self) -> bool {
        f64::is_infinite(*self)
    }
}

pub fn parse_real<T: ParsableReal>(base: NumBase, number: &str) -> NumberResult<T> {
    let radix = T::from(base.radix());
    let mut accum = T::from(0);
    let mut numbers_since_dot = None::<i32>;
    let mut excess_precision = 0u32;
    for (i, c) in number.as_bytes().iter().enumerate() {
        match base.digit_to_value(*c) {
            Some(0) => {
                accum *= radix.clone();
                numbers_since_dot = numbers_since_dot.map(|f| f.saturating_add(1));
            },
            Some(raw_digit) => {
                let digit = T::from(raw_digit);
                accum *= radix.clone();
                let before_addition = accum.clone();
                accum += digit;
                numbers_since_dot = numbers_since_dot.map(|f| f.saturating_add(1));
                if before_addition >= accum {
                    excess_precision += 1;
                }
            },
            None if *c == b'.' => {
                if numbers_since_dot.is_none() {
                    numbers_since_dot = Some(0);
                } else {
                    return Err(ParseNumberError::SecondDot { index: i });
                }
            },
            None => {
                return Err(ParseNumberError::NonDigitByte { byte: *c, index: i });
            },
        }
    }

    if let Some(numbers_since_dot) = numbers_since_dot {
        accum /= radix.pow(numbers_since_dot);
    }
    let overflowed = accum.is_infinity();
    Ok(ParsedNumber {
        number: accum,
        overflowed,
        excess_precision,
    })
}
/// A type alias representing a parsed number or a parse error.
pub type NumberResult<T> = std::result::Result<ParsedNumber<T>, ParseNumberError>;
/// The struct that contains the parsed number and any extra flags about the result.
pub struct ParsedNumber<T> {
    /// The number that was parsed. Depending on the number, this may be incorrect.
    pub number: T,
    /// Whether extra numbers occured in a float. These numbers have no bearing on the actual number.
    /// Always 0 when returned from [parse_int].
    pub excess_precision: u32,
    /// Whether the number 'overflowed'.
    /// * [parse_int] will wrap the parsed number around when this occurs.
    /// * [parse_real] will have an infinity value when this occurs.
    pub overflowed: bool,
}

#[derive(Debug)]
pub enum ParseNumberError {
    NonDigitByte { byte: u8, index: usize },
    SecondDot { index: usize },
}
impl fmt::Display for ParseNumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ParseNumberError::NonDigitByte { byte, index } => {
                if byte.is_ascii() {
                    write!(
                        f,
                        "The character '{}' occured at index {}. This character is not a digit for the base.",
                        byte as char, index
                    )
                } else {
                    write!(
                        f,
                        "An unknown byte occured at index {}. Numbers should not contain any special characters or Unicode.",
                        index
                    )
                }
            },
            ParseNumberError::SecondDot { index } => {
                write!(
                    f,
                    "A second dot occured at index {}. Only up to one dot can be in a real number.",
                    index
                )
            },
        }
    }
}
impl std::error::Error for ParseNumberError {}

#[cfg(test)]
mod tests {
    // The floating point comparisons are meant to be exact.
    #![allow(clippy::clippy::float_cmp)]

    use super::*;
    type TestResult = std::result::Result<(), ParseNumberError>;

    #[test]
    fn parse_integer_parses_correctly() -> TestResult {
        let test_cases = [
            ("101", 5, NumBase::Binary),
            ("777", 511, NumBase::Octal),
            ("0", 0, NumBase::Decimal),
            ("30000", 30000, NumBase::Decimal),
            ("CAFE", 0xCAFE, NumBase::Hexadecimal),
        ];
        for &(number, expected, base) in &test_cases {
            let result = parse_int::<usize>(base, number)?;
            assert_eq!(
                result.number, expected,
                "'{}' (base {:?}) parsed incorrectly!",
                number, base
            );
        }
        Ok(())
    }

    #[test]
    fn parse_integer_overflows_correctly() -> TestResult {
        let test_cases = [
            ("100000000000000000000000000000001", 1, NumBase::Binary),
            ("40000000007", 7, NumBase::Octal),
            ("4294967299", 3, NumBase::Decimal),
            ("FFFFFFFFF", 0xFFFFFFFF, NumBase::Hexadecimal),
        ];
        for &(number, expected, base) in &test_cases {
            let result = parse_int::<u32>(base, number)?;
            assert!(
                result.overflowed,
                "'{}' (base {:?}) should have overflowed!",
                number, base
            );
            assert_eq!(
                result.number, expected,
                "'{}' (base {:?}) parsed incorrectly!",
                number, base
            );
        }
        Ok(())
    }

    #[test]
    fn parse_float_correctly() -> TestResult {
        let test_cases = [
            ("1", 1.0, NumBase::Binary),
            ("1.", 1.0, NumBase::Binary),
            (".", 0.0, NumBase::Binary),
            ("1.1", 1.5, NumBase::Binary),
            ("1.4", 1.5, NumBase::Octal),
            ("1.1", 1.1, NumBase::Decimal),
            ("1000000.5", 1000000.5, NumBase::Decimal),
        ];
        for &(number, expected, base) in &test_cases {
            let result = parse_real::<f32>(base, number)?;
            assert_eq!(
                result.number, expected,
                "'{}' (base {:?}) parsed incorrectly!",
                number, base
            );
        }
        Ok(())
    }

    #[test]
    fn parse_float_overflows_correctly() -> TestResult {
        // There are 38 9s in this literal
        let test_case = "399999999999999999999999999999999999999";
        let result = parse_real::<f32>(NumBase::Decimal, test_case)?;
        assert!(
            result.overflowed,
            "'{}' should have overflowed, not produced: {}",
            test_case, result.number
        );
        Ok(())
    }

    #[test]
    fn parse_float_excess_precision_is_correct() -> TestResult {
        let test_case = "4.0000000000000000000000000000000000000000000000000000000323";
        let result = parse_real::<f32>(NumBase::Decimal, test_case)?;
        assert_eq!(
            result.excess_precision, 3,
            "'{}' should have had 3 digits of excess precision.",
            test_case
        );
        Ok(())
    }
}

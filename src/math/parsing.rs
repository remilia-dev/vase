// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

use crate::math::{
    Integer,
    NumBase,
    Number,
    Real,
};
/// Attempts to parse a numerical string (containing no dot).
/// If the numerical string contains a non-digit character, an error
/// will be returned.
///
/// See [NumBase::parse_int] for more information.
pub(super) fn parse_int<T>(base: NumBase, number: &[u8]) -> NumberResult<T>
where T: Integer {
    let mut res = ParsedNumber {
        number: T::from(0),
        overflowed: false,
        excess_precision: 0,
    };
    let radix = T::from(base.radix());
    for (i, &c) in number.iter().enumerate() {
        if let Some(raw_digit) = base.digit_to_value(c) {
            let digit = T::from(raw_digit);
            let mut overflow;
            (res.number, overflow) = res.number.overflowing_mul(radix);
            res.overflowed |= overflow;
            (res.number, overflow) = res.number.overflowing_add(digit);
            res.overflowed |= overflow;
        } else {
            return Err(ParseNumberError {
                before_error: res,
                base,
                real: false,
                invalid_byte: c,
                index: i,
            });
        }
    }

    Ok(res)
}
/// Attempts to parse a numerical string (potentially containing a dot).
/// If the numerical string contains a non-digit character (for
/// this base), an error will be returned.
///
/// See [NumBase::parse_real] for more info.
pub(super) fn parse_real<T>(base: NumBase, number: &[u8]) -> NumberResult<T>
where T: Real {
    let mut res = ParsedNumber {
        number: T::from(0),
        overflowed: false,
        excess_precision: 0,
    };
    let radix = T::from(base.radix());
    let mut numbers_since_dot = None::<i32>;
    let mut i = 0;
    while i < number.len() {
        let c = number[i];
        match base.digit_to_value(c) {
            Some(0) => {
                res.number *= radix;
                numbers_since_dot = numbers_since_dot.map(|f| f.saturating_add(1));
            },
            Some(raw_digit) => {
                let digit = T::from(raw_digit);
                res.number *= radix;
                let before_addition = res.number;
                res.number += digit;
                numbers_since_dot = numbers_since_dot.map(|f| f.saturating_add(1));
                if before_addition >= res.number {
                    res.excess_precision += 1;
                    i += 1;
                    break;
                }
            },
            None if c == b'.' => {
                if numbers_since_dot.is_none() {
                    numbers_since_dot = Some(0);
                } else {
                    break;
                }
            },
            None => break,
        }
        i += 1;
    }

    if res.excess_precision == 1 {
        // Verify that everything past the excess precision is a valid digit.
        while i < number.len() {
            let c = number[i];
            if c == b'.' && numbers_since_dot.is_none() {
                numbers_since_dot = Some(0);
            } else if base.is_digit_ascii(c) {
                res.excess_precision += 1;
                if numbers_since_dot.is_none() {
                    res.number *= radix;
                }
            } else {
                break;
            }
            i += 1;
        }
    }

    if let Some(numbers_since_dot) = numbers_since_dot {
        res.number /= radix.powi(numbers_since_dot);
    }
    res.overflowed = !res.number.is_finite();

    if i == number.len() {
        Ok(res)
    } else {
        Err(ParseNumberError {
            before_error: res,
            base,
            real: true,
            index: i,
            invalid_byte: number[i],
        })
    }
}
/// The struct that contains the parsed number and any extra flags about the result.
#[derive(Clone, Debug)]
pub struct ParsedNumber<N> {
    /// The number that was parsed. Depending on the number, this may be incorrect.
    pub number: N,
    /// The number of excess non-0 digits in a float.
    /// These numbers have no bearing on the actual number.
    /// Always 0 when returned from [NumBase::parse_int].
    pub excess_precision: u32,
    /// Whether the number 'overflowed'.
    /// * [NumBase::parse_int] will wrap the parsed number around when this occurs.
    /// * [NumBase::parse_real] will have an infinity value when this occurs.
    pub overflowed: bool,
}
/// An error that has resulted from an invalid digit while parsing a number.
#[derive(Debug)]
pub struct ParseNumberError<N: Number> {
    /// The parsed number *so-far*.
    before_error: ParsedNumber<N>,
    /// The base that was being parsed.
    base: NumBase,
    /// Whether [NumBase::parse_real] was called or [NumBase::parse_int].
    real: bool,
    /// The invalid byte that caused the error.
    invalid_byte: u8,
    /// The index the invalid byte occured at.
    /// This is also how far into the string the parsing got.
    index: usize,
}

impl<N: Number> fmt::Display for ParseNumberError<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.invalid_byte {
            b'.' if self.real => write!(
                f,
                "A second . occured in the number at byte {}",
                self.index,
            ),
            c if c.is_ascii() => write!(
                f,
                "The ASCII character {} at index {} is not a valid digit character for the base {}",
                c as char, self.index, self.base,
            ),
            c => write!(
                f,
                "The non-ASCII byte {} occured at index {}. Only ASCII characters can be used in numbers.",
                c, self.index,
            ),
        }
    }
}

impl<N: Number> std::error::Error for ParseNumberError<N> {}
/// A type alias representing a parsed number or a parse error.
pub type NumberResult<N> = std::result::Result<ParsedNumber<N>, ParseNumberError<N>>;

#[cfg(test)]
mod tests {
    // The floating point comparisons are meant to be exact.
    #![allow(clippy::clippy::float_cmp)]

    use super::*;
    type TestResult<N> = std::result::Result<(), ParseNumberError<N>>;

    #[test]
    fn parse_integer_parses_correctly() -> TestResult<u32> {
        let test_cases = [
            ("101", 5, NumBase::Binary),
            ("777", 511, NumBase::Octal),
            ("0", 0, NumBase::Decimal),
            ("30000", 30000, NumBase::Decimal),
            ("CAFE", 0xCAFE, NumBase::Hexadecimal),
        ];
        for &(number, expected, base) in &test_cases {
            let result = base.parse_int::<u32, _>(&number)?;
            assert_eq!(
                result.number, expected,
                "'{}' (base {:?}) parsed incorrectly!",
                number, base
            );
        }
        Ok(())
    }

    #[test]
    fn parse_integer_overflows_correctly() -> TestResult<u32> {
        let test_cases = [
            ("100000000000000000000000000000001", 1, NumBase::Binary),
            ("40000000007", 7, NumBase::Octal),
            ("4294967299", 3, NumBase::Decimal),
            ("FFFFFFFFF", 0xFFFFFFFF, NumBase::Hexadecimal),
        ];
        for &(number, expected, base) in &test_cases {
            let result = base.parse_int::<u32, _>(&number)?;
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
    fn parse_float_correctly() -> TestResult<f32> {
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
            let result = base.parse_real::<f32, _>(&number)?;
            assert_eq!(
                result.number, expected,
                "'{}' (base {:?}) parsed incorrectly!",
                number, base
            );
        }
        Ok(())
    }

    #[test]
    fn parse_float_overflows_correctly() -> TestResult<f32> {
        // There are 38 9s in this literal
        let test_case = "399999999999999999999999999999999999999";
        let result = NumBase::Decimal.parse_real::<f32, _>(&test_case)?;
        assert!(
            result.overflowed,
            "'{}' should have overflowed, not produced: {}",
            test_case, result.number
        );
        Ok(())
    }

    #[test]
    fn parse_float_excess_precision_is_correct() -> TestResult<f32> {
        let test_case = "4.0000000000000000000000000000000000000000000000000000000323";
        let result = NumBase::Decimal.parse_real::<f32, _>(&test_case)?;
        assert_eq!(
            result.excess_precision, 3,
            "'{}' should have had 3 digits of excess precision.",
            test_case
        );
        Ok(())
    }
}

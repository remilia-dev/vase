// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryFrom;

use crate::{
    c::StringEnc,
    error::{
        CodedError,
        ErrorReceiver,
        MayUnwind,
        Severity,
    },
    math::{
        Integer,
        NumBase,
        ParsedNumber,
        Real,
    },
    util::{
        create_intos,
        enum_with_properties,
        CharExt,
        SourceLoc,
    },
};

#[derive(Clone, Debug)]
pub struct Literal {
    pub loc: SourceLoc,
    pub kind: LiteralKind,
}

#[create_intos]
#[derive(Clone, Debug)]
pub enum LiteralKind {
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl LiteralKind {
    pub fn is_real(&self) -> bool {
        matches!(self, Self::F32(..) | Self::F64(..))
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(self, Self::U32(..) | Self::U64(..))
    }

    pub fn is_zero(&self) -> bool {
        match *self {
            Self::I32(0) | Self::U32(0) | Self::I64(0) | Self::U64(0) => true,
            Self::F32(f) => f == 0.0,
            Self::F64(f) => f == 0.0,
            _ => false,
        }
    }

    pub fn from_number<D>(digits: D, on_error: LiteralReceiver) -> MayUnwind<LiteralKind>
    where D: AsRef<[u8]> {
        LiteralDecoder::create_and_calc(digits.as_ref(), on_error)
    }

    pub fn from_character<C: AsRef<str>>(
        chars: C,
        encoding: StringEnc,
        errors: LiteralReceiver,
    ) -> MayUnwind<LiteralKind> {
        parse_character(chars.as_ref(), encoding, errors)
    }
}

pub type LiteralReceiver<'a> = &'a mut dyn ErrorReceiver<LiteralError>;
enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum LiteralError {
        // == Errors
        #[values(Error, 600)]
        EmptyNumber,
        #[values(Error, 601)]
        EmptyExponent,
        #[values(Error, 602)]
        InvalidIntSuffix(String),
        #[values(Error, 603)]
        InvalidRealSuffix(String),
        #[values(Error, 610)]
        InvalidEscape(Option<char>),
        #[values(Error, 611)]
        ExtraChars(usize),
        #[values(Error, 612)]
        CharTooBigForEncoding(u32, StringEnc),
        #[values(Error, 613)]
        UnicodeEscapeMissingDigits(u32),
        // == Warnings
        #[values(Warning, 300)]
        OverflowOccured(bool),
        #[values(Warning, 301)]
        ExcessPrecision(u32),
        #[values(Warning, 310)]
        CharOverflowed,
    }

    impl CodedError for LiteralError {
        #[property]
        fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        fn code_number(&self) -> u32 {}

        fn code_prefix(&self) -> &'static str {
            // NOTE: The lexer prefix is use since these errors are lexical in nature.
            // The code numbers for this error should be unique with respect to LexerError.
            "C-L"
        }

        fn message(&self) -> String {
            use LiteralError::*;
            match *self {
                // == Errors
                EmptyNumber => "This number has no digits.".to_owned(),
                EmptyExponent => "The exponent of this number has no digits.".to_owned(),
                InvalidIntSuffix(ref suffix) => format!(
                    "'{}' is not a valid suffix for an integer number.",
                    suffix
                ),
                InvalidRealSuffix(ref suffix) => format!(
                    "'{}' is not a valid suffix for a real number.",
                    suffix
                ),
                InvalidEscape(maybe) => match maybe {
                    Some(char) => format!(
                        "\\{} is not a valid escape sequence",
                        char
                    ),
                    None => "Something has to follow a backslash in a character constant.".to_owned()
                },
                ExtraChars(count) => format!(
                    "There are {} excess characters in the literal. These are ignored.",
                    count
                ),
                CharTooBigForEncoding(char, encoding) => format!(
                    "The character literal has a value of {} but the literal's encoding only supports up to {}.",
                    char, encoding.mask()
                ),
                UnicodeEscapeMissingDigits(count) => format!(
                    "The unicode character escape expects {} more hexadecimal digits.",
                    count
                ),
                // == Warnings
                OverflowOccured(is_exp) => format!(
                    "Overflow occured while parsing this number{}.",
                    if is_exp { "'s exponent" } else { "" }
                ),
                ExcessPrecision(digits) => format!(
                    "The last {} digits have no effect on the number.",
                    digits
                ),
                CharOverflowed => "Overflow occured while parsing \\x escape.".to_owned(),
            }
        }
    }
}

struct LiteralDecoder<'a> {
    errors: LiteralReceiver<'a>,
    base: NumBase,
    number: &'a [u8],
    has_dot: bool,
    exp_base: Option<u8>,
    negative_exp: bool,
    exp: &'a [u8],
    suffix: &'a [u8],
}

impl<'a> LiteralDecoder<'a> {
    fn create_and_calc(number: &'a [u8], errors: LiteralReceiver<'a>) -> MayUnwind<LiteralKind> {
        Self::new(number, errors).calc_number()
    }

    fn new(number: &'a [u8], errors: LiteralReceiver<'a>) -> Self {
        let mut prefix_length = 0;
        let mut base = NumBase::Decimal;
        if number.get(0) == Some(&b'0') {
            (base, prefix_length) = match number.get(1) {
                Some(b'x') => (NumBase::Hexadecimal, 2),
                Some(b'b') => (NumBase::Binary, 2),
                Some(&c) if c.is_ascii_digit() => (NumBase::Octal, 1),
                _ => (NumBase::Decimal, 0),
            };
        }

        let number = &number[prefix_length..];

        let (number_len, has_dot) = base.find_end_of_digits(number, true);
        let (number, post_number) = number.split_at(number_len);

        let exp_base = match post_number.get(0) {
            Some(b'e' | b'E') => Some(10),
            Some(b'p' | b'P') => Some(2),
            _ => None,
        };

        if exp_base.is_some() {
            let (negative_exp, post_number) = match post_number.get(1) {
                Some(b'-') => (true, &post_number[2..]),
                Some(b'+') => (false, &post_number[2..]),
                _ => (false, &post_number[1..]),
            };
            let (exp_len, _) = NumBase::Decimal.find_end_of_digits(post_number, false);

            let (exp, suffix) = post_number.split_at(exp_len);
            Self {
                errors,
                base,
                number,
                has_dot,
                exp_base,
                negative_exp,
                exp,
                suffix,
            }
        } else {
            Self {
                errors,
                base,
                number,
                has_dot,
                exp_base,
                negative_exp: false,
                exp: b"",
                suffix: post_number,
            }
        }
    }

    fn calc_number(&mut self) -> MayUnwind<LiteralKind> {
        self.report_empty_segments()?;
        let suffix = self.decode_suffix()?;

        match suffix {
            SuffixType::DefaultInt(force_long) if self.base == NumBase::Decimal => {
                let l_value = self.parse_int::<i64>()?;
                if !force_long {
                    if let Ok(value) = i32::try_from(l_value) {
                        return Ok(value.into());
                    }
                }
                Ok(l_value.into())
            },
            SuffixType::DefaultInt(force_long) => {
                let l_value = self.parse_int::<u64>()?;
                if !force_long {
                    if let Ok(i) = i32::try_from(l_value) {
                        return Ok(i.into());
                    } else if let Ok(u) = u32::try_from(l_value) {
                        return Ok(u.into());
                    }
                }
                if let Ok(i) = i64::try_from(l_value) {
                    Ok(i.into())
                } else {
                    Ok(l_value.into())
                }
            },
            SuffixType::UnsignedInt(force_long) => {
                let l_value = self.parse_int::<u64>()?;
                if !force_long {
                    if let Ok(value) = u32::try_from(l_value) {
                        return Ok(value.into());
                    }
                }
                Ok(l_value.into())
            },
            SuffixType::Float => {
                let value = self.parse_real::<f32>()?;
                Ok(value.into())
            },
            SuffixType::Double => {
                let value = self.parse_real::<f64>()?;
                Ok(value.into())
            },
            _ => {
                // TODO: C23 decimal literals
                eprintln!("Decimal reals have not been implemented yet.");
                unimplemented!()
            },
        }
    }

    fn decode_suffix(&mut self) -> MayUnwind<SuffixType> {
        if self.has_dot || self.exp_base.is_some() {
            match self.suffix {
                b"" | b"l" | b"L" => Ok(SuffixType::Double),
                b"f" | b"F" => Ok(SuffixType::Float),
                b"df" | b"DF" => Ok(SuffixType::Decimal32),
                b"dd" | b"DD" => Ok(SuffixType::Decimal64),
                b"dl" | b"DL" => Ok(SuffixType::Decimal128),
                _ => {
                    self.report_invalid_suffix()?;
                    Ok(SuffixType::Double)
                },
            }
        } else {
            let mut u_count = 0;
            let mut l_count = 0;
            for &c in self.suffix {
                match c {
                    b'u' | b'U' => u_count += 1,
                    b'l' | b'L' => l_count += 1,
                    _ => {},
                }
            }

            if u_count > 1 || u_count + l_count != self.suffix.len() {
                self.report_invalid_suffix()?;
            } else if u_count == 1 && l_count == 2 {
                let u_pos = self.suffix.iter().position(|x| *x == b'u').unwrap();
                if u_pos == 1 {
                    // `lul` suffix is not a valid suffix.
                    self.report_invalid_suffix()?;
                }
            } else if l_count > 2 {
                self.report_invalid_suffix()?;
            }

            if u_count > 0 {
                Ok(SuffixType::UnsignedInt(l_count > 0))
            } else {
                Ok(SuffixType::DefaultInt(l_count > 0))
            }
        }
    }

    fn parse_int<T: Integer>(&mut self) -> MayUnwind<T> {
        let parsed = self.unwrap_parsed(
            self.base.parse_int::<T, _>(self.number).unwrap(), //
            false,
        )?;
        // self.exp_base will always be None since numbers with
        // exponents are read a floating point numbers.
        Ok(parsed)
    }

    fn parse_real<T: Real>(&mut self) -> MayUnwind<T> {
        let mut parsed = self.unwrap_parsed(
            self.base.parse_real::<T, _>(self.number).unwrap(), //
            false,
        )?;
        if let Some(exp_base) = self.exp_base {
            let exp_base = T::from(exp_base);
            let mut exp = self.unwrap_parsed(
                self.base.parse_int::<i32, _>(self.exp).unwrap(), //
                true,
            )?;
            if self.negative_exp {
                exp *= -1;
            }
            parsed *= exp_base.powi(exp);
        }
        Ok(parsed)
    }

    fn unwrap_parsed<N>(&mut self, parsed: ParsedNumber<N>, exponent: bool) -> MayUnwind<N> {
        if parsed.overflowed {
            self.errors.report(LiteralError::OverflowOccured(exponent))?;
        }
        if parsed.excess_precision != 0 {
            let error = LiteralError::ExcessPrecision(parsed.excess_precision);
            self.errors.report(error)?;
        }
        Ok(parsed.number)
    }

    fn report_invalid_suffix(&mut self) -> MayUnwind<()> {
        let suffix = String::from_utf8(self.suffix.into()).unwrap();
        if self.has_dot {
            self.errors.report(LiteralError::InvalidRealSuffix(suffix))
        } else {
            self.errors.report(LiteralError::InvalidIntSuffix(suffix))
        }
    }

    fn report_empty_segments(&mut self) -> MayUnwind<()> {
        if self.number.is_empty() {
            self.errors.report(LiteralError::EmptyNumber)?;
        }
        if self.exp_base.is_some() && self.suffix.is_empty() {
            self.errors.report(LiteralError::EmptyExponent)?;
        }
        Ok(())
    }
}

enum SuffixType {
    DefaultInt(bool),
    UnsignedInt(bool),
    Double,
    Float,
    Decimal32,
    Decimal64,
    Decimal128,
}

pub fn parse_character(
    chars: &str,
    encoding: StringEnc,
    errors: LiteralReceiver,
) -> MayUnwind<LiteralKind> {
    let (char, used) = if chars.as_bytes().get(0) == Some(&b'\\') {
        match chars.as_bytes().get(1) {
            Some(b'\'') => ('\'' as u32, 2),
            Some(b'"') => ('"' as u32, 2),
            Some(b'?') => ('?' as u32, 2),
            Some(b'\\') => ('\\' as u32, 2),
            Some(b'a') => ('\u{7}' as u32, 2),
            Some(b'b') => ('\u{8}' as u32, 2),
            Some(b'f') => ('\u{C}' as u32, 2),
            Some(b'n') => ('\n' as u32, 2),
            Some(b'r') => ('\r' as u32, 2),
            Some(b't') => ('\t' as u32, 2),
            Some(b'v') => ('\u{B}' as u32, 2),
            Some(&c) if c.is_ascii_octdigit() => {
                let mut result = parse_complex_character(
                    NumBase::Octal, //
                    &chars[1..],
                    3,
                    errors,
                )?;
                result.1 += 1;
                result
            },
            Some(b'x') => {
                let mut result = parse_complex_character(
                    NumBase::Hexadecimal, //
                    &chars[2..],
                    usize::MAX,
                    errors,
                )?;
                result.1 += 2;
                result
            },
            Some(b'u') | Some(b'U') => {
                let max = if chars.as_bytes()[1] == b'u' { 4 } else { 8 };
                let mut result = parse_complex_character(
                    NumBase::Hexadecimal, //
                    &chars[2..],
                    max,
                    errors,
                )?;
                if result.1 != max {
                    let missing = (max - result.1) as u32;
                    errors.report(LiteralError::UnicodeEscapeMissingDigits(missing))?;
                }
                result.1 += 2;
                result
            },
            _ => {
                let char = chars.chars().nth(1);
                errors.report(LiteralError::InvalidEscape(char))?;
                (char.map_or(0, |c| c as u32), chars.len())
            },
        }
    } else {
        (chars.chars().next().unwrap() as u32, 1)
    };

    if used < chars.len() {
        errors.report(LiteralError::ExtraChars(chars.len() - used))?;
    }

    let mask = encoding.mask();
    if char & !mask != 0 {
        errors.report(LiteralError::CharTooBigForEncoding(char, encoding))?;
        Ok(((char & mask) as i32).into())
    } else {
        Ok((char as i32).into())
    }
}

fn parse_complex_character(
    base: NumBase,
    chars: &str,
    max_digits: usize,
    errors: LiteralReceiver,
) -> MayUnwind<(u32, usize)> {
    let (mut digit_count, _) = base.find_end_of_digits(chars, false);
    digit_count = digit_count.min(max_digits);
    let digits = &chars[..digit_count];
    let parsed = base.parse_int::<u32, _>(digits).unwrap();

    if parsed.overflowed {
        errors.report(LiteralError::CharOverflowed)?;
    }

    Ok((parsed.number, digit_count))
}

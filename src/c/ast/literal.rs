// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryFrom;

use crate::{
    c::StringEncoding,
    error::{
        CodedError,
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

    pub fn from_number<D, E>(digits: D, on_error: E) -> MayUnwind<LiteralKind>
    where
        D: AsRef<[u8]>,
        E: OnLiteralError,
    {
        LiteralDecoder::create_and_calc(digits.as_ref(), on_error)
    }

    pub fn from_character<C, E>(
        chars: C,
        _encoding: StringEncoding,
        _on_error: E,
    ) -> MayUnwind<LiteralKind>
    where
        C: AsRef<str>,
        E: OnLiteralError,
    {
        // TODO: Character literal handling
        let chars = chars.as_ref();
        let _char = if chars.as_bytes().get(0) == Some(&b'\\') {
            match chars.as_bytes().get(1) {
                Some(b'\'') => '\'' as u32,
                Some(b'"') => '"' as u32,
                Some(b'?') => '?' as u32,
                Some(b'\\') => '\\' as u32,
                Some(b'a') => '\u{7}' as u32,
                Some(b'b') => '\u{8}' as u32,
                Some(b'f') => '\u{C}' as u32,
                Some(b'n') => '\n' as u32,
                Some(b'r') => '\r' as u32,
                Some(b't') => '\t' as u32,
                Some(b'v') => '\u{B}' as u32,
                Some(&c) if c.is_ascii_octdigit() => {
                    unimplemented!()
                },
                Some(b'x') => {
                    unimplemented!()
                },
                Some(b'u') => {
                    unimplemented!()
                },
                Some(b'U') => {
                    unimplemented!()
                },
                _ => {
                    unimplemented!()
                },
            }
        } else {
            chars.chars().next().unwrap() as u32
        };
        unimplemented!()
    }
}

pub trait OnLiteralError = FnMut(LiteralError) -> MayUnwind<()>;
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
        // == Warnings
        #[values(Warning, 300)]
        OverflowOccured(bool),
        #[values(Warning, 301)]
        ExcessPrecision(u32),
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
    }
}

struct LiteralDecoder<'a, E: OnLiteralError> {
    on_error: E,
    base: NumBase,
    number: &'a [u8],
    has_dot: bool,
    exp_base: Option<u8>,
    negative_exp: bool,
    exp: &'a [u8],
    suffix: &'a [u8],
}

impl<'a, E: OnLiteralError> LiteralDecoder<'a, E> {
    fn create_and_calc(number: &'a [u8], on_error: E) -> MayUnwind<LiteralKind> {
        Self::new(number, on_error).calc_number()
    }

    fn new(number: &'a [u8], on_error: E) -> Self {
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
                on_error,
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
                on_error,
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
            (self.on_error)(LiteralError::OverflowOccured(exponent))?;
        }
        if parsed.excess_precision != 0 {
            (self.on_error)(LiteralError::ExcessPrecision(parsed.excess_precision))?;
        }
        Ok(parsed.number)
    }

    fn report_invalid_suffix(&mut self) -> MayUnwind<()> {
        let suffix = String::from_utf8(self.suffix.into()).unwrap();
        if self.has_dot {
            (self.on_error)(LiteralError::InvalidRealSuffix(suffix))
        } else {
            (self.on_error)(LiteralError::InvalidIntSuffix(suffix))
        }
    }

    fn report_empty_segments(&mut self) -> MayUnwind<()> {
        if self.number.is_empty() {
            (self.on_error)(LiteralError::EmptyNumber)?;
        }
        if self.exp_base.is_some() && self.suffix.is_empty() {
            (self.on_error)(LiteralError::EmptyExponent)?;
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

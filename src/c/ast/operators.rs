// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

use crate::{
    c::TokenKind,
    util::enum_with_properties,
};

enum_with_properties! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
    pub enum Precedence {
        #[values(None)]
        Atoms,
        #[values(LeftToRight)]
        Suffixes,
        #[values(RightToLeft)]
        Prefixes,
        #[values(LeftToRight)]
        Multiplicative,
        #[values(LeftToRight)]
        Additive,
        #[values(LeftToRight)]
        Shifting,
        #[values(LeftToRight)]
        Relational,
        #[values(LeftToRight)]
        Equality,
        #[values(LeftToRight)]
        BitAnd,
        #[values(LeftToRight)]
        BitXor,
        #[values(LeftToRight)]
        BitOr,
        #[values(LeftToRight)]
        LogicalAnd,
        #[values(LeftToRight)]
        LogicalOr,
        #[values(RightToLeft)]
        Ternary,
        #[values(RightToLeft)]
        Assignment,
        #[values(LeftToRight)]
        Comma,
    }

    impl Precedence {
        #[property]
        pub fn associativity(self) -> Associativity {
            use Associativity::*;
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Associativity {
    LeftToRight,
    RightToLeft,
    None,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PrefixOp {
    Increment,
    Decrement,
    Posate,
    Negate,
    LogicalNot,
    BitNot,
    Dereference,
    AddressOf,
    SizeOf,
    AlignOf,
}

impl std::convert::TryFrom<&TokenKind> for PrefixOp {
    type Error = ();

    fn try_from(value: &TokenKind) -> Result<Self, Self::Error> {
        use TokenKind::*;

        use crate::c::Keyword::{
            Alignof,
            Sizeof,
        };

        Ok(match *value {
            PlusPlus => Self::Increment,
            MinusMinus => Self::Decrement,
            Plus => Self::Posate,
            Minus => Self::Negate,
            Bang => Self::LogicalNot,
            Tilde => Self::BitNot,
            Star => Self::Dereference,
            Amp => Self::AddressOf,
            Keyword(Sizeof) => Self::SizeOf,
            Keyword(Alignof) => Self::AlignOf,
            _ => return Err(()),
        })
    }
}

enum_with_properties! {
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub enum BinaryOp {
        #[values("*", Multiplicative)]
        Multiplication,
        #[values("/", Multiplicative)]
        Divide,
        #[values("%", Multiplicative)]
        Modulo,
        #[values("+", Additive)]
        Addition,
        #[values("-", Additive)]
        Subtraction,
        #[values("<<", Shifting)]
        LShift,
        #[values(">>", Shifting)]
        RShift,
        #[values("<", Relational)]
        LessThan,
        #[values("<=", Relational)]
        LessThanOrEqual,
        #[values(">", Relational)]
        GreaterThan,
        #[values(">=", Relational)]
        GreaterThanOrEqual,
        #[values("==", Equality)]
        Equals,
        #[values("!=", Equality)]
        NotEquals,
        #[values("&", BitAnd)]
        BitAnd,
        #[values("^", BitXor)]
        BitXor,
        #[values("|", BitOr)]
        BitOr,
        #[values("&&", LogicalAnd)]
        LogicalAnd,
        #[values("||", LogicalOr)]
        LogicalOr,
    }

    impl BinaryOp {
        #[property]
        pub fn text(self) -> &'static str {}
        #[property]
        pub fn precedence(self) -> Precedence {
            use Precedence::*;
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text())
    }
}

impl std::convert::TryFrom<&TokenKind> for BinaryOp {
    type Error = ();

    fn try_from(value: &TokenKind) -> Result<Self, Self::Error> {
        use TokenKind::*;
        Ok(match *value {
            Star => Self::Multiplication,
            Slash => Self::Divide,
            Percent => Self::Modulo,
            Plus => Self::Addition,
            Minus => Self::Subtraction,
            LShift => Self::LShift,
            RShift => Self::RShift,
            LAngle => Self::LessThan,
            LAngleEqual => Self::LessThanOrEqual,
            RAngle => Self::GreaterThan,
            RAngleEqual => Self::GreaterThanOrEqual,
            EqualEqual => Self::Equals,
            BangEqual => Self::NotEquals,
            Amp => Self::BitAnd,
            Carrot => Self::BitXor,
            Bar => Self::BitOr,
            AmpAmp => Self::LogicalAnd,
            BarBar => Self::LogicalOr,
            _ => return Err(()),
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AssignOp {
    None,
    Multiplication,
    Divide,
    Modulo,
    Addition,
    Subtraction,
    LShift,
    RShift,
    BitAnd,
    BitXor,
    BitOr,
}

impl std::convert::TryFrom<&TokenKind> for AssignOp {
    type Error = ();

    fn try_from(kind: &TokenKind) -> Result<Self, Self::Error> {
        use TokenKind::*;
        Ok(match *kind {
            Equal => Self::None,
            StarEqual => Self::Multiplication,
            SlashEqual => Self::Divide,
            PercentEqual => Self::Modulo,
            PlusEqual => Self::Addition,
            MinusEqual => Self::Subtraction,
            LShiftEqual => Self::LShift,
            RShiftEqual => Self::RShift,
            AmpEqual => Self::BitAnd,
            CarrotEqual => Self::BitXor,
            BarEqual => Self::BitOr,
            _ => return Err(()),
        })
    }
}

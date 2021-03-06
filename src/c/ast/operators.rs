// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
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
            Keyword(Sizeof, ..) => Self::SizeOf,
            Keyword(Alignof, ..) => Self::AlignOf,
            _ => return Err(()),
        })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    Multiplication,
    Divide,
    Modulo,
    Addition,
    Subtraction,
    LShift,
    RShift,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Equals,
    NotEquals,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr,
}

impl BinaryOp {
    pub fn precedence(self) -> Precedence {
        use BinaryOp::*;
        match self {
            Multiplication | Divide | Modulo => Precedence::Multiplicative,
            Addition | Subtraction => Precedence::Additive,
            LShift | RShift => Precedence::Shifting,
            LessThan | LessThanOrEqual | GreaterThan | GreaterThanOrEqual => Precedence::Relational,
            Equals | NotEquals => Precedence::Equality,
            BitAnd => Precedence::BitAnd,
            BitXor => Precedence::BitXor,
            BitOr => Precedence::BitOr,
            LogicalAnd => Precedence::LogicalAnd,
            LogicalOr => Precedence::LogicalOr,
        }
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
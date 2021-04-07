// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        CompileSettings,
        LangVersion,
    },
    util::{
        variant_list,
        variant_names,
    },
};

#[variant_list]
#[variant_names]
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Keyword {
    Auto,
    Break,
    Case,
    Char,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extern,
    Float,
    For,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Register,
    Restrict,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    Struct,
    Switch,
    Typedef,
    Union,
    Unsigned,
    Void,
    Volatile,
    While,
    Alignas,
    Alignof,
    Atomic,
    Bool,
    Complex,
    Decimal32,
    Decimal64,
    Decimal128,
    Generic,
    Imaginary,
    Noreturn,
    Pragma,
    StaticAssert,
    ThreadLocal,
}

impl Keyword {
    pub fn text(self) -> &'static str {
        use Keyword::*;
        match self {
            Auto => "auto",
            Break => "break",
            Case => "case",
            Char => "char",
            Const => "const",
            Continue => "continue",
            Default => "default",
            Do => "do",
            Double => "double",
            Else => "else",
            Enum => "enum",
            Extern => "extern",
            Float => "float",
            For => "for",
            Goto => "goto",
            If => "if",
            Inline => "inline",
            Int => "int",
            Long => "long",
            Register => "register",
            Restrict => "restrict",
            Return => "return",
            Short => "short",
            Signed => "signed",
            Sizeof => "sizeof",
            Static => "static",
            Struct => "struct",
            Switch => "switch",
            Typedef => "typedef",
            Union => "union",
            Unsigned => "unsigned",
            Void => "void",
            Volatile => "volatile",
            While => "while",
            Alignas => "_Alignas",
            Alignof => "_Alignof",
            Atomic => "_Atomic",
            Bool => "_Bool",
            Complex => "_Complex",
            Decimal32 => "_Decimal32",
            Decimal64 => "_Decimal64",
            Decimal128 => "_Decimal128",
            Generic => "_Generic",
            Imaginary => "_Imaginary",
            Noreturn => "_Noreturn",
            Pragma => "_Pragma",
            StaticAssert => "_Static_assert",
            ThreadLocal => "_Thread_local",
        }
    }

    pub fn should_add(self, settings: &CompileSettings) -> bool {
        match self {
            Self::Inline | Self::Restrict => settings.version >= LangVersion::C99,
            _ => true,
        }
    }

    pub fn is_type_starter(self) -> bool {
        self.is_base_type() | self.is_type_modifier() | self.is_storage_class() | self.is_type_tag()
    }

    pub fn is_type_modifier(self) -> bool {
        matches!(
            self,
            Self::Const
                | Self::Inline
                | Self::Long
                | Self::Short
                | Self::Signed
                | Self::Unsigned
                | Self::Volatile
                | Self::Atomic
                | Self::Complex
                | Self::Imaginary
                | Self::Noreturn
                | Self::ThreadLocal
        )
    }

    pub fn is_storage_class(self) -> bool {
        matches!(
            self,
            Self::Auto | Self::Static | Self::Extern | Self::Register | Self::Typedef
        )
    }

    pub fn is_base_type(self) -> bool {
        matches!(
            self,
            Self::Bool
                | Self::Char
                | Self::Int
                | Self::Float
                | Self::Double
                | Self::Void
                | Self::Decimal32
                | Self::Decimal64
                | Self::Decimal128
        )
    }

    pub fn is_type_tag(self) -> bool {
        matches!(self, Self::Struct | Self::Union)
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    sync::Arc,
    util::CachedString,
};

#[derive(Debug)]
pub struct CToken {
    byte: u32,
    byte_length: u32,
    kind: CTokenKind,
    whitespace_before: bool,
}
impl CToken {
    pub fn new(byte: u32, byte_length: u32, kind: CTokenKind, whitespace_before: bool) -> CToken {
        CToken {
            byte,
            byte_length,
            kind,
            whitespace_before,
        }
    }

    pub fn byte(&self) -> u32 {
        self.byte
    }
    pub fn byte_length(&self) -> u32 {
        self.byte_length
    }
    pub fn kind(&self) -> &CTokenKind {
        &self.kind
    }
    pub fn kind_mut(&mut self) -> &mut CTokenKind {
        &mut self.kind
    }
    pub fn whitespace_before(&self) -> bool {
        self.whitespace_before
    }
}

#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CTokenKind {
    IncludePath {
        inc_type: CIncludeType,
        path: CachedString,
    },
    Message(Arc<Box<str>>),
    Identifier(CachedString),
    Number {
        num_type: CNumberType,
        num_data: CachedString,
    },
    String {
        str_type: CStringType,
        has_complex_escapes: bool,
        is_char: bool,
        str_data: Arc<Box<str>>,
    },
    EOF,

    // == Begin Preprocessors
    PreIf {
        link: usize,
    },
    PreIfDef {
        link: usize,
    },
    PreIfNDef {
        link: usize,
    },
    PreElif {
        link: usize,
    },
    PreElse {
        link: usize,
    },
    PreEndIf,
    PreDefine,
    PreUndef,
    PreLine,
    PreError,
    PrePragma,
    PreBlank,
    PreInclude,
    // Other
    PreEnd,
    PreUnknown(CachedString),
    // GCC Extensions
    PreIncludeNext,
    PreWarning,
    // == End Preprocessors

    // == Begin Keywords
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
    Decimal32,
    Decimal64,
    Decimal128,
    Complex,
    Generic,
    Imaginary,
    Noreturn,
    Pragma,
    StaticAssert,
    ThreadLocal,
    // == End Keywords

    // == Begin Symbols
    /// `[` when alt is false
    ///
    /// `<:` when alt is true
    LBracket {
        alt: bool,
    },
    /// `]` when alt is false
    ///
    /// `:>` when alt is true
    RBracket {
        alt: bool,
    },
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{` when alt is false
    ///
    /// `<%` when alt is true
    LBrace {
        alt: bool,
    },
    /// `}` when alt is false
    ///
    /// `%>` when alt is true
    RBrace {
        alt: bool,
    },

    /// `&`
    Amp,
    /// `&=`
    AmpEqual,
    /// `&&`
    AmpAmp,
    /// `->`
    Arrow,
    /// `@`
    At,
    /// `\`
    Backslash,
    /// `!`
    Bang,
    /// `!=`
    BangEqual,
    /// `|`
    Bar,
    /// `|=`
    BarEqual,
    /// `||`
    BarBar,
    /// `^`
    Carrot,
    /// `^=`
    CarrotEqual,
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `...`
    DotDotDot,
    /// `=`
    Equal,
    /// `==`
    EqualEqual,
    /// `#` when alt is false
    ///
    /// `%:` when alt is true
    Hash {
        alt: bool,
    },
    /// `##` when alt is false
    ///
    /// `%:%:` when alt is true
    HashHash {
        alt: bool,
    },
    /// `-`
    Minus,
    /// `-=`
    MinusEqual,
    /// `--`
    MinusMinus,
    /// `<`
    LAngle,
    /// `<=`
    LAngleEqual,
    /// `<<`
    LShift,
    /// `<<=`
    LShiftEqual,
    /// `%`
    Percent,
    /// `%=`
    PercentEqual,
    /// `+`
    Plus,
    /// `+=`
    PlusEqual,
    /// `++`
    PlusPlus,
    /// `?`
    QMark,
    /// `>`
    RAngle,
    /// `>=`
    RAngleEqual,
    /// `>>`
    RShift,
    /// `>>=`
    RShiftEqual,
    /// `;`
    Semicolon,
    /// `/`
    Slash,
    /// `/=`
    SlashEqual,
    /// `*`
    Star,
    /// `*=`
    StarEqual,
    /// `~`
    Tilde,
    // == End Symbols
}
impl CTokenKind {
    pub fn is_linking(&self) -> bool {
        return matches!(
            self,
            CTokenKind::PreIf { .. }
                | CTokenKind::PreIfDef { .. }
                | CTokenKind::PreIfNDef { .. }
                | CTokenKind::PreElif { .. }
                | CTokenKind::PreElse { .. }
        );
    }

    pub fn ends_a_link(&self) -> bool {
        return matches!(
            self,
            CTokenKind::PreElse { .. } | CTokenKind::PreElif { .. } | CTokenKind::PreEndIf { .. }
        );
    }

    pub fn set_link(&mut self, val: usize) {
        match self {
            CTokenKind::PreIf { link }
            | CTokenKind::PreIfDef { link }
            | CTokenKind::PreIfNDef { link }
            | CTokenKind::PreElif { link }
            | CTokenKind::PreElse { link } => *link = val,
            _ => {},
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CIncludeType {
    IncludeSystem, // For #include <file>
    IncludeLocal,  // For #include "file"
    IncludeNext,   // For #include_next "file"
}
impl CIncludeType {
    pub fn is_end_char(&self, c: char) -> bool {
        match c {
            '"' => *self == CIncludeType::IncludeLocal,
            '>' => *self == CIncludeType::IncludeSystem,
            _ => false,
        }
    }

    pub fn check_relative(&self) -> bool {
        return matches!(
            self,
            CIncludeType::IncludeLocal | CIncludeType::IncludeNext
        );
    }

    pub fn ignore_own_file(&self) -> bool {
        return matches!(self, CIncludeType::IncludeNext);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum CNumberType {
    // NOTE: Types need to be in increasing number of digits.
    Bin,
    Oct,
    Dec,
    Hex,
}
impl CNumberType {
    pub fn supports_digit(self, c: char) -> bool {
        match c {
            '0' | '1' => true,
            '2' | '3' | '4' | '5' | '6' | '7' => self != CNumberType::Bin,
            '8' | '9' => self >= CNumberType::Dec,
            'a' | 'A' | 'b' | 'B' | 'c' | 'C' | 'd' | 'D' | 'e' | 'E' | 'f' | 'F' => {
                self == CNumberType::Hex
            },
            _ => false,
        }
    }

    pub fn supports_exp(self, c: char) -> bool {
        match c {
            'e' | 'E' => self != CNumberType::Hex,
            'p' | 'P' => self == CNumberType::Hex,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CStringType {
    Default,
    U8,
    WChar,
    U16,
    U32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_token_is_at_most_32_bytes() {
        // Testing limits the size of CToken since even small size increases will result in
        // higher memory usage (and not by a tiny amount).
        let size = std::mem::size_of::<CToken>();
        assert!(
            size <= 32,
            "CToken is {} bytes when it should be 32 or less.",
            size
        );
    }
}

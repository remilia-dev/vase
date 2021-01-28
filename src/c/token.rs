use crate::{
    sync::Arc,
    util::{
        CachedString,
        SourceLocation,
    },
};

#[derive(Debug)]
pub struct CToken {
    loc: SourceLocation,
    kind: CTokenKind,
    length: u32,
}
impl CToken {
    pub fn new(loc: SourceLocation, length: u32, kind: CTokenKind) -> CToken {
        CToken { loc, length, kind }
    }

    pub fn kind(&self) -> &CTokenKind {
        &self.kind
    }
    pub fn kind_mut(&mut self) -> &mut CTokenKind {
        &mut self.kind
    }
    pub fn length(&self) -> u32 {
        self.length
    }
}

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum CTokenKind {
    Preprocessor(CPreprocessorType),
    UnknownPreprocessor(CachedString),
    PreprocessorEnd,
    IncludePath {
        inc_type: CIncludeType,
        path: CachedString,
    },
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
    LParen {
        whitespace_before: bool,
    },
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
    /// `.`
    Dot,
    /// `->`
    Arrow,
    /// `++`
    PlusPlus,
    /// `--`
    MinusMinus,
    /// `&`
    Amp,
    /// `*`
    Star,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `~`
    Tilde,
    /// `!`
    Bang,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `<<`
    LShift,
    /// `>>`
    RShift,
    /// `<`
    LAngle,
    /// `>`
    RAngle,
    /// `<=`
    LAngleEqual,
    /// `>=`
    RAngleEqual,
    /// `==`
    EqualEqual,
    /// `!=`
    BangEqual,
    /// `^`
    Carrot,
    /// `|`
    Bar,
    /// `&&`
    AmpAmp,
    /// `||`
    BarBar,
    /// `?`
    QMark,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `...`
    DotDotDot,
    /// `=`
    Equal,
    /// `*=`
    StarEqual,
    /// `/=`
    SlashEqual,
    /// `%=`
    PercentEqual,
    /// `+=`
    PlusEqual,
    /// `-=`
    MinusEqual,
    /// `<<=`
    LShiftEqual,
    /// `>>=`
    RShiftEqual,
    /// `&=`
    AmpEqual,
    /// `^=`
    CarrotEqual,
    /// `|=`
    BarEqual,
    /// `,`
    Comma,
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
    /// `@`
    At,
    /// `\`
    Backslash,
    // == End Symbols
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CPreprocessorType {
    If { link: u32 },
    IfDef { link: u32 },
    IfNDef { link: u32 },
    Elif { link: u32 },
    Else { link: u32 },
    EndIf,
    Define,
    Undef,
    Line,
    Error,
    Pragma,
    Blank,
    Include,
    IncludeNext, // GCC Extension
    Warning, // GCC Extension
    // ## MISSING:
    // SCCS and IDENT (Unofficial GCC Extension)
    // Assert/Unassert (Deprecated GCC Extension)
}
impl CPreprocessorType {
    pub fn is_include(&self) -> bool {
        return matches!(
            self,
            CPreprocessorType::Include | CPreprocessorType::IncludeNext
        );
    }

    pub fn is_linking(&self) -> bool {
        return matches!(
            self,
            CPreprocessorType::If { .. }
                | CPreprocessorType::IfDef { .. }
                | CPreprocessorType::IfNDef { .. }
                | CPreprocessorType::Elif { .. }
                | CPreprocessorType::Else { .. }
        );
    }

    pub fn ends_a_link(&self) -> bool {
        return matches!(
            self,
            CPreprocessorType::Else { .. }
                | CPreprocessorType::Elif { .. }
                | CPreprocessorType::EndIf { .. }
        );
    }

    pub fn set_link(&mut self, val: u32) {
        match self {
            CPreprocessorType::If { link }
            | CPreprocessorType::IfDef { link }
            | CPreprocessorType::IfNDef { link }
            | CPreprocessorType::Elif { link }
            | CPreprocessorType::Else { link } => *link = val,
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

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum CStringType {
    DEFAULT,
    U8,
    WCHAR,
    U16,
    U32,
}

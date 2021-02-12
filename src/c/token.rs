// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    sync::Arc,
    util::CachedString,
};

#[derive(Clone, Debug)]
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

    pub fn new_unknown(kind: CTokenKind) -> CToken {
        CToken {
            byte: u32::MAX,
            byte_length: u32::MAX,
            whitespace_before: true,
            kind,
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
    Number(CachedString),
    String {
        str_type: CStringType,
        has_complex_escapes: bool,
        is_char: bool,
        str_data: Arc<Box<str>>,
    },
    Eof,

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
        use CTokenKind::*;
        matches!(
            self,
            PreIf { .. } | PreIfDef { .. } | PreIfNDef { .. } | PreElif { .. } | PreElse { .. }
        )
    }

    pub fn ends_a_link(&self) -> bool {
        use CTokenKind::*;
        matches!(self, PreElse { .. } | PreElif { .. } | PreEndIf { .. })
    }

    pub fn set_link(&mut self, val: usize) {
        use CTokenKind::*;
        match *self {
            PreIf { ref mut link }
            | PreIfDef { ref mut link }
            | PreIfNDef { ref mut link }
            | PreElif { ref mut link }
            | PreElse { ref mut link } => *link = val,
            _ => {},
        }
    }

    pub fn is_keyword(&self) -> bool {
        use CTokenKind::*;
        // NOTE: The comments are there to keep rustfmt happy.
        matches!(
            self,
            Auto | Break | Case | Char | Const | Continue | Default | Do | Double | Else | Enum // 1
            | Extern | Float | For | Goto | If | Inline | Int | Long | Register | Restrict // 2
            | Return | Short | Signed | Sizeof | Static | Struct | Switch | Typedef | Union // 3
            | Unsigned | Void | Volatile | While | Alignas | Alignof | Atomic | Bool // 4
            | Decimal32 | Decimal64 | Decimal128 | Complex | Generic | Imaginary | Noreturn // 5
            | Pragma | StaticAssert | ThreadLocal // 6
        )
    }

    /// Is able to be joined using ## with another token that is id-joinable.
    ///
    /// For example `int ## ID` is joinable to produce the identifier `intId`.
    pub fn is_id_joinable(&self) -> bool {
        self.is_keyword() || matches!(self, CTokenKind::Identifier(..) | CTokenKind::Number(..))
    }

    pub fn get_id_join_text(&self) -> &str {
        use CTokenKind::*;
        match *self {
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
            Identifier(ref id) => id.string(),
            Number(ref num) => num.string(),
            _ => panic!(
                "get_id_joinable_text should only be used on tokens that are is_id_joinable."
            ),
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
    pub fn is_end_char(self, c: char) -> bool {
        match c {
            '"' => self == CIncludeType::IncludeLocal,
            '>' => self == CIncludeType::IncludeSystem,
            _ => false,
        }
    }

    pub fn check_relative(self) -> bool {
        return matches!(
            self,
            CIncludeType::IncludeLocal | CIncludeType::IncludeNext
        );
    }

    pub fn ignore_own_file(self) -> bool {
        return matches!(self, CIncludeType::IncludeNext);
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

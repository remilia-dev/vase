// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

use crate::{
    c::FileId,
    sync::Arc,
    util::{
        CachedString,
        SourceLocation,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    location: SourceLocation,
    whitespace_before: bool,
    kind: TokenKind,
}
impl Token {
    pub fn new(location: SourceLocation, whitespace_before: bool, kind: TokenKind) -> Token {
        Token { location, whitespace_before, kind }
    }

    pub fn new_first_byte(file_id: FileId, kind: TokenKind) -> Token {
        Token {
            location: SourceLocation::new_first_byte(file_id),
            whitespace_before: false,
            kind,
        }
    }

    pub fn location(&self) -> &SourceLocation {
        &self.location
    }
    pub fn whitespace_before(&self) -> bool {
        self.whitespace_before
    }
    pub fn kind(&self) -> &TokenKind {
        &self.kind
    }
    pub fn kind_mut(&mut self) -> &mut TokenKind {
        &mut self.kind
    }
}

#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    IncludePath {
        inc_type: IncludeType,
        path: CachedString,
    },
    // OPTIMIZATION: Remove the excess Box (See String too). This would involve using some thin-dst type.
    Message(Arc<Box<str>>),
    Identifier(CachedString),
    Keyword(Keyword, usize),
    Number(CachedString),
    String {
        encoding: StringEncoding,
        has_escapes: bool,
        is_char: bool,
        str_data: Arc<Box<str>>,
    },
    LexerError(usize),
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
impl TokenKind {
    pub fn is_linking(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            PreIf { .. } | PreIfDef { .. } | PreIfNDef { .. } | PreElif { .. } | PreElse { .. }
        )
    }

    pub fn ends_a_link(&self) -> bool {
        use TokenKind::*;
        matches!(self, PreElse { .. } | PreElif { .. } | PreEndIf { .. })
    }

    pub fn set_link(&mut self, val: usize) {
        use TokenKind::*;
        match *self {
            PreIf { ref mut link }
            | PreIfDef { ref mut link }
            | PreIfNDef { ref mut link }
            | PreElif { ref mut link }
            | PreElse { ref mut link } => *link = val,
            _ => {},
        }
    }

    /// Is able to be joined using ## with another token to form a number.
    pub fn is_number_joinable_with(&self, other: &TokenKind) -> bool {
        use TokenKind::*;
        match *self {
            Dot => matches!(*other, Number { .. }),
            Number(ref digits) => match *other {
                Number { .. } | Identifier(..) | Dot => true,
                Plus | Minus => matches!(
                    digits.string().as_bytes().last(),
                    Some(b'e' | b'E' | b'p' | b'P')
                ),
                _ => false,
            },
            _ => false,
        }
    }

    /// Is able to be joined using ## with another token to form an identifier
    /// (or potentially a keyword).
    ///
    /// For example `int ## ID` is joinable to produce the identifier `intID`.
    pub fn is_id_joinable_with(&self, other: &TokenKind) -> bool {
        use TokenKind::*;
        matches!(self, Identifier(..) | Keyword(..))
            & matches!(other, Identifier(..) | Keyword(..) | Number { .. })
    }

    /// Gets the token's simple textual form.
    /// # Panics
    /// Panics if this token would require allocations to represent its textual form.
    /// For example, a String token will panic because the quotes would need to be
    /// added.
    pub fn text(&self) -> &str {
        use TokenKind::*;
        match *self {
            Message(ref message) => message,
            Identifier(ref id) => id.string(),
            Keyword(keyword, ..) => keyword.text(),
            Number(ref digits) => digits.string(),
            PreIf { .. } => "#if",
            PreIfDef { .. } => "#ifdef",
            PreIfNDef { .. } => "#ifndef",
            PreElif { .. } => "#elif",
            PreElse { .. } => "#else",
            PreEndIf => "#endif",
            PreDefine => "#define",
            PreUndef => "#undefine",
            PreLine => "#line",
            PreError => "#error",
            PrePragma => "#pragma",
            PreBlank => "#",
            PreInclude => "#include",
            PreIncludeNext => "#include_next",
            PreWarning => "#warning",
            LBracket { alt } => (if alt { "<:" } else { "[" }),
            RBracket { alt } => (if alt { ":>" } else { "]" }),
            LParen => "(",
            RParen => ")",
            LBrace { alt } => (if alt { "<%" } else { "{" }),
            RBrace { alt } => (if alt { "%>" } else { "}" }),
            Amp => "&",
            AmpEqual => "&=",
            AmpAmp => "&&",
            Arrow => "->",
            At => "@",
            Backslash => "\\",
            Bang => "!",
            BangEqual => "!=",
            Bar => "|",
            BarEqual => "|=",
            BarBar => "||",
            Carrot => "^",
            CarrotEqual => "^=",
            Colon => ":",
            Comma => ",",
            Dot => ".",
            DotDotDot => "...",
            Equal => "=",
            EqualEqual => "==",
            Hash { alt } => (if alt { "%:" } else { "#" }),
            HashHash { alt } => (if alt { "%:%:" } else { "##" }),
            Minus => "-",
            MinusEqual => "-=",
            MinusMinus => "--",
            LAngle => "<",
            LAngleEqual => "<=",
            LShift => "<<",
            LShiftEqual => "<<=",
            Percent => "%",
            PercentEqual => "%=",
            Plus => "+",
            PlusEqual => "+=",
            PlusPlus => "++",
            QMark => "?",
            RAngle => ">",
            RAngleEqual => ">=",
            RShift => ">>",
            RShiftEqual => ">>=",
            Semicolon => ";",
            Slash => "/",
            SlashEqual => "/=",
            Star => "*",
            StarEqual => "*=",
            Tilde => "~",
            _ => panic!(
                "Token that does not have a corresponding text representation: {:?}",
                self
            ),
        }
    }

    pub fn is_definable(&self) -> bool {
        use TokenKind::*;
        matches!(self, Identifier(..) | Keyword(..))
    }

    pub fn get_definable_id(&self) -> usize {
        use TokenKind::*;
        match *self {
            Identifier(ref id) => id.uniq_id(),
            Keyword(_, unique_id) => unique_id,
            _ => panic!(
                "get_definable_unique_id should only be used on tokens that are is_definable."
            ),
        }
    }

    pub fn is_preprocessor(&self) -> bool {
        // PreBlank isn't treated like a preprocessor because it isn't followed by a PreEnd.
        use TokenKind::*;
        matches!(
            *self,
            // Comments are to make rustfmt happy.
            PreIf { .. } | PreIfDef { .. } | PreIfNDef { .. } | PreElif { .. } | PreElse { .. } // 1
            | PreEndIf | PreDefine | PreUndef | PreLine | PreError | PrePragma | PreInclude // 2
            | PreUnknown(..) | PreIncludeNext | PreWarning // 3
        )
    }
}
impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TokenKind::*;
        match *self {
            IncludePath { inc_type, ref path } => {
                if inc_type.check_relative() {
                    write!(f, r#""{}""#, path)
                } else {
                    write!(f, "<{}>", path)
                }
            },
            String { encoding, is_char, ref str_data, .. } => {
                let prefix = encoding.prefix().unwrap_or("");
                if is_char {
                    write!(f, "{}'{}'", prefix, str_data)
                } else {
                    write!(f, r#"{}"{}""#, prefix, str_data)
                }
            },
            PreUnknown(ref instr) => write!(f, "#{}", instr),
            LexerError(..) | Eof | PreEnd => Ok(()),
            _ => write!(f, "{}", self.text()),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum IncludeType {
    IncludeSystem, // For #include <file>
    IncludeLocal,  // For #include "file"
    IncludeNext,   // For #include_next "file"
}
impl IncludeType {
    pub fn is_end_char(self, c: char) -> bool {
        match c {
            '"' => self == IncludeType::IncludeLocal,
            '>' => self == IncludeType::IncludeSystem,
            _ => false,
        }
    }

    pub fn check_relative(self) -> bool {
        return matches!(self, IncludeType::IncludeLocal | IncludeType::IncludeNext);
    }

    pub fn ignore_own_file(self) -> bool {
        return matches!(self, IncludeType::IncludeNext);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StringEncoding {
    Default,
    U8,
    WChar,
    U16,
    U32,
}
impl StringEncoding {
    pub fn prefix(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::U8 => Some("u8"),
            Self::WChar => Some("L"),
            Self::U16 => Some("u"),
            Self::U32 => Some("U"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_token_is_at_most_32_bytes() {
        // Testing limits the size of CToken since even small size increases will result in
        // higher memory usage (and not by a tiny amount).
        let size = std::mem::size_of::<Token>();
        assert!(
            size <= 32,
            "CToken is {} bytes when it should be 32 or less.",
            size
        );
    }
}

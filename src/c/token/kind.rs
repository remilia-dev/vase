// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::fmt;

use crate::{
    c::{
        IncludeType,
        Keyword,
        StringEnc,
    },
    sync::Arc,
    util::{
        variant_names,
        CachedString,
    },
};

#[variant_names]
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
    Keyword(Keyword),
    Number(CachedString),
    String {
        encoding: StringEnc,
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

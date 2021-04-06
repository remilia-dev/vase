// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::{
            BinaryExpr,
            NumberError,
            PrefixExpr,
        },
        IncludeType,
        LexerError,
        Token,
        TokenKind,
        TravelerState,
    },
    error::{
        CodedError,
        Severity,
    },
    math::Sign,
    sync::Arc,
    util::{
        enum_with_properties,
        CachedString,
        FileId,
        SourceLoc,
    },
};

#[derive(Clone, Debug)]
pub struct TravelerError {
    pub state: TravelerState,
    pub kind: TravelerErrorKind,
}

impl CodedError for TravelerError {
    fn severity(&self) -> Severity {
        self.kind.severity()
    }

    fn code_number(&self) -> u32 {
        self.kind.code_number()
    }

    fn code_prefix(&self) -> &'static str {
        self.kind.code_prefix()
    }

    fn message(&self) -> String {
        self.kind.message()
    }
}

enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum TravelerErrorKind {
        // == Others
        #[values(v0.severity(), v0.code_number())]
        Lexer(LexerError),
        #[values(v0.severity(), v0.code_number())]
        Number(NumberError),
        // == Internals
        #[values(Internal, 900)]
        Unimplemented(&'static str),
        #[values(Internal, 901)]
        Unreachable(&'static str),
        // == Fatals
        #[values(Fatal, 800)]
        ErrorPreprocessor(Option<Arc<Box<str>>>),
        #[values(Fatal, 850)]
        IncludeNotFound(Option<FileId>, IncludeType, CachedString),
        // == Errors
        #[values(Error, 500)]
        IfDefExpectedId(Token, Token),
        #[values(Error, 501)]
        IfDefExtraTokens(Token),
        #[values(Error, 510)]
        IfExpectedAtom(Token, Token),
        #[values(Error, 511)]
        IfExpectedOp(Token, Token),
        #[values(Error, 512)]
        IfDefinedNotDefinable(Token, bool, Token),
        #[values(Error, 513)]
        IfDefinedExpectedRParen(Token, Token),
        #[values(Error, 514)]
        IfExpectedRParen(Token, Token),
        #[values(Error, 515)]
        IfTernaryExpectedColon(Token, Token),
        #[values(Error, 516)]
        IfDiv0(Token, Sign, Box<BinaryExpr>),
        #[values(Error, 517)]
        IfReal(Token, Token),
        #[values(Error, 520)]
        ElseExtraTokens,
        #[values(Error, 521)]
        EndIfExtraTokens,
        #[values(Error, 530)]
        DefineExpectedId(Token),
        #[values(Error, 531)]
        DefineFuncEndBeforeEndOfArgs,
        #[values(Error, 532)]
        DefineFuncExpectedArg(Token),
        #[values(Error, 533)]
        DefineFuncExpectedSeparator(Token),
        #[values(Error, 534)]
        DefineFuncExpectedEndOfArgs(Token),
        #[values(Error, 540)]
        UndefExpectedId(Token),
        #[values(Error, 541)]
        UndefExtraTokens,
        #[values(Error, 550)]
        IncludeExpectedPath(Token),
        #[values(Error, 551)]
        IncludeExtraTokens,
        #[values(Error, 560)]
        FuncInvokeMissingArgs(usize),
        #[values(Error, 561)]
        FuncInvokeExcessParameters(Vec<Token>),
        #[values(Error, 562)]
        FuncInvokePreprocessorInArgs(Token),
        #[values(Error, 563)]
        InnerFuncInvokeUnfinished,
        #[values(Error, 564)]
        StringifyExpectsId(Token),
        #[values(Error, 565)]
        StringifyNonParameter(Token),
        #[values(Error, 570)]
        InvalidJoin(Token, SourceLoc, Token),
        #[values(Error, 580)]
        StrayHash,
        #[values(Error, 581)]
        StrayHashHash,
        #[values(Error, 582)]
        StrayAtSign,
        #[values(Error, 583)]
        StrayBackslash,
        #[values(Error, 590)]
        UnknownPreprocessor(CachedString),
        // == Warning
        #[values(Warning, 210)]
        CommaInIfCondition,
        #[values(Warning, 211)]
        OverflowInIfBinary(i64, i64, Box<BinaryExpr>),
        #[values(Warning, 212)]
        OverflowInIfNegation(i64, Box<PrefixExpr>),
        #[values(Warning, 213)]
        NegativeSignedToUnsigned(bool, i64, Box<BinaryExpr>),
        #[values(Warning, 214)]
        ShiftedToMuch(Sign, Sign, Box<BinaryExpr>),
        #[values(Warning, 280)]
        WarningPreprocessor(Option<Arc<Box<str>>>),
        #[values(Warning, 299)]
        UnsupportableLinePreprocessor,
    }

    impl CodedError for TravelerErrorKind {
        #[property]
        fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        fn code_number(&self) -> u32 {}

        fn code_prefix(&self) -> &'static str {
            match *self {
                Self::Lexer(ref error) => error.code_prefix(),
                Self::Number(ref error) => error.code_prefix(),
                _ => "C-T",
            }
        }

        fn message(&self) -> String {
            // NOTE: See the end of this file for the messages
            Self::message(self)
        }
    }
}

impl From<LexerError> for TravelerErrorKind {
    fn from(error: LexerError) -> Self {
        TravelerErrorKind::Lexer(error)
    }
}

impl From<NumberError> for TravelerErrorKind {
    fn from(error: NumberError) -> Self {
        TravelerErrorKind::Number(error)
    }
}

impl TravelerErrorKind {
    // The function should be consistently formatted. Rustfmt tries to destroy
    // this consistency by putting some things on single lines.
    #[rustfmt::skip]
    // The lines are just a result of the number of errors.
    #[allow(clippy::clippy::too_many_lines)]
    fn message(&self) -> String {
        use TravelerErrorKind::*;
        match *self {
            // == Others
            Lexer(ref error) => error.message(),
            Number(ref error) => error.message(),
            // == Internals
            Unimplemented(thing) => format!(
                "{} is currently unimplemented.",
                thing
            ),
            Unreachable(thing) => format!(
                "Unreachable condition: {}. This is an internal error.",
                thing
            ),
            // == Fatals
            ErrorPreprocessor(ref message) => format!(
                "#error: {}",
                message.as_ref().map_or("", |message| &*message)
            ),
            IncludeNotFound(_,kind, ref path) => format!(
                "A {} of the path {} could not be found.",
                kind, path
            ),
            // == Errors
            IfDefExpectedId(ref ifdef, ref bad_token) => match *bad_token.kind() {
                TokenKind::PreEnd => format!(
                    "{} expects an identifier to follow on the same line. None was found.",
                    ifdef
                ),
                TokenKind::Number(..) => format!(
                    "{} would not expect a number to follow since numbers cannot be macros.",
                    ifdef
                ),
                _ => format!(
                    "{} expects an identifier of a potential macro (not a {}).",
                    ifdef, bad_token
                ),
            },
            IfDefExtraTokens(ref ifdef) => format!(
                "Only a single identifier should follow {}.",
                ifdef
            ),
            IfExpectedAtom(_, ref token) => format!(
                "Expected a number, an identifier, a left parenthesis, \
                or a unary operator (+, -, ~, !) (not a {}).",
                token
            ),
            IfExpectedOp(_, ref token) => format!(
                "Expected an operator (*, /, %, etc.) (not a {}).",
                token
            ),
            IfDefinedNotDefinable(_, has_parens, ref token) => match *token.kind() {
                TokenKind::PreEnd => {
                    "The defined preprocessor operator should be followed by an identifier."
                        .to_owned()
                },
                TokenKind::Number(..) => {
                    "The defined preprocessor operator would not expect a number \
                    since numbers cannot be macros."
                        .to_owned()
                },
                TokenKind::RParen if has_parens => {
                    "The defined preprocessor operator should have an identifier \
                    between the parenthesis."
                        .to_owned()
                },
                _ => format!(
                    "The defined preprocessor operator would expect an identifier here (not a {}).",
                    token
                ),
            },
            IfDefinedExpectedRParen(_, ref token) => format!(
                "Since the defined preprocessor operation was started with a (, \
                it should be ended with a ) (not a {}).",
                token
            ),
            IfExpectedRParen(_, ref token) => match *token.kind() {
                TokenKind::PreEnd => {
                    "A corresponding end ) was expected before the end of the line.".to_owned()
                },
                TokenKind::Colon => {
                    "A ternary : was found before a corresponding end ).".to_owned()
                },
                _ => format!(
                    "A corresponding ) was expected (not a {}).",
                    token
                ),
            },
            IfTernaryExpectedColon(_, ref token) => match *token.kind() {
                TokenKind::PreEnd => {
                    "Ternary operator's : was not found before the end of the \
                    preprocessor condition.".to_owned()
                },
                TokenKind::RParen => {
                    "A unbalanced ) was found where a ternary operator's : should be.".to_owned()
                },
                _ => format!(
                    "A ternary oeprator's : separator was expected (found {}).",
                    token
                ),
            },
            IfDiv0(_, ref left, ref expr) => format!(
                "Division by zero occured in preprocessor condition: {} {} 0",
                left, expr.op
            ),
            IfReal(ref if_token, ..) => format!(
                "Real numbers are not allowed in {} conditions. Only integers can be used.",
                if_token
            ),
            ElseExtraTokens => {
                "#else should not be followed by anything on the same line.".to_owned()
            },
            EndIfExtraTokens => {
                "#endif should not be followed by anything on the same line.".to_owned()
            },
            DefineExpectedId(ref token) => match *token.kind() {
                TokenKind::PreEnd => {
                    "#define expects an identifier to follow on the same line. None was found."
                        .to_owned()
                },
                TokenKind::Number(..) => {
                    "A macro can't be defined starting with a number."
                        .to_owned()
                },
                _ => format!(
                    "#define expects the identifer of the macro (not a {}).",
                    token
                ),
            },
            DefineFuncEndBeforeEndOfArgs => {
                "An end-of-line occured before the end of a func-macro's arguments.".to_owned()
            },
            DefineFuncExpectedArg(ref token) => {
                if let TokenKind::Number(..) = *token.kind() {
                    "A function macro argument can't be a number. It must start with a non-digit."
                        .to_owned()
                } else {
                    format!(
                        "Expected a function macro argument name or ... (not a {}).",
                        token
                    )
                }
            },
            DefineFuncExpectedSeparator(ref token) => {
                if token.kind().is_definable() {
                    "A comma should separate the function macro arguments.".to_owned()
                } else {
                    format!(
                        "Expected a ) to end the parameters, a comma to separate parameters, \
                        or a ... to make a var-arg (not a {}).",
                        token
                    )
                }
            },
            DefineFuncExpectedEndOfArgs(ref token) => {
                if let TokenKind::Comma = *token.kind() {
                    "No other parameters can be defined after a var-arg. \
                    Did you mean to end the arguments with a )?"
                        .to_owned()
                } else {
                    format!(
                        "Expected a ) to end the function macro parameters \
                        since the last parameter is a var-arg (not a {}).",
                        token
                    )
                }
            },
            UndefExpectedId(ref token) => match *token.kind() {
                TokenKind::PreEnd => {
                    "#undef expects an identifer of a macro to undefine to follow \
                    on the same line. None was found."
                        .to_owned()
                },
                TokenKind::Number(..) => {
                    "#undef would not expect a number to follow it since numbers cannot be macros."
                        .to_owned()
                },
                _ => format!(
                    "#undef expects an identifier of a macro to undefine (not a {}).",
                    token
                ),
            },
            UndefExtraTokens => {
                "#undef expects only a single identifier to follow it on the same line.".to_owned()
            },
            IncludeExpectedPath(ref path) => match *path.kind() {
                TokenKind::PreEnd => {
                    "Expected an include path before the end of the line.".to_owned()
                },
                _ => format!(
                    "Expected an include path (not a {}).",
                    path
                )
            },
            IncludeExtraTokens => {
                "Only a single include path should follow an include directive.".to_owned()
            },
            FuncInvokeMissingArgs(count) => format!(
                "This func-macro invocation is missing {} parameters.",
                count
            ),
            FuncInvokeExcessParameters(..) => {
                "This func-macro invocation has more parameters than the macro has.".to_owned()
            },
            FuncInvokePreprocessorInArgs(..) => {
                "Preprocessor instructions have undefined behavior in function parameters."
                    .to_owned()
            },
            InnerFuncInvokeUnfinished => {
                "A function macro that started inside a parameter is without an end parenthesis."
                    .to_owned()
            },
            StringifyExpectsId(ref token) => format!(
                "Can't stringify {} since it isn't an identifier of \
                a parameter to this function macro.",
                token
            ),
            StringifyNonParameter(ref token) => format!(
                "Can't stringify {} as it isn't a parameter to this function macro.",
                token
            ),
            InvalidJoin(ref left, _, ref right) => format!(
                "{} ## {} does not produce a valid token ({0}{1} has to be a single token).",
                left, right
            ),
            StrayHash => {
                "A stray # was found in the program. # can only be used in function macros."
                    .to_owned()
            },
            StrayHashHash => {
                "A stray ## was found in the program. ## can only be used in a macro.".to_owned()
            },
            StrayAtSign => {
                "A stray @ was found in the program. @ are not used in C.".to_owned()
            },
            StrayBackslash => {
                "A stray \\ was found in the program.".to_owned()
            },
            UnknownPreprocessor(ref instruction) => format!(
                "'#{}' is an unknown preprocessor instruction.",
                instruction
            ),
            // == Warnings
            CommaInIfCondition => {
                "The comma operator discards everything before it in the conditional.".to_owned()
            },
            OverflowInIfBinary(lhs, rhs, ref expr) => format!(
                "{} {} {} results in an undefined, signed overflow.",
                lhs, expr.op, rhs
            ),
            OverflowInIfNegation(number, ..) => format!(
                "-{} results in undefined, signed overflow.",
                number
            ),
            NegativeSignedToUnsigned(rhs, ..) => format!(
                "Undefined negative-signed-to-unsigned conversion on the {} side of the operator.",
                if rhs { "right" } else { "left" }
            ),
            ShiftedToMuch(ref left, ref right, ref expr) => format!(
                "{} {} {} is undefined due to the right value being larger than 63 or negative.",
                left, expr.op, right
            ),
            WarningPreprocessor(ref message) => format!(
                "#warning: {}",
                message.as_ref().map_or("", |message| &*message)
            ),
            UnsupportableLinePreprocessor => {
                "Due to this compiler's design, #line is unsupportable.".to_owned()
            },
        }
    }
}

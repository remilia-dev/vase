// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::{
            BinaryExpr,
            LiteralError,
            PrefixExpr,
        },
        IncludeType,
        LexerError,
        Token,
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
}

enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum TravelerErrorKind {
        // == Others
        #[values(v0.severity(), v0.code_number())]
        LexerError(LexerError),
        #[values(v0.severity(), v0.code_number())]
        LiteralError(LiteralError),
        // == Internals
        #[values(Internal, 900)]
        Unimplemented(&'static str),
        #[values(Internal, 901)]
        Unreachable(&'static str),
        #[values(Internal, 902)]
        MissingIncludeId(FileId),
        // == Fatals
        #[values(Error, 800)]
        ErrorPreprocessor(Option<Arc<Box<str>>>),
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
        #[values(Error, 552)]
        IncludeNotFound(IncludeType, CachedString),
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
                Self::LexerError(ref error) => error.code_prefix(),
                Self::LiteralError(ref error) => error.code_prefix(),
                _ => "C-T",
            }
        }
    }
}
impl From<LexerError> for TravelerErrorKind {
    fn from(error: LexerError) -> Self {
        TravelerErrorKind::LexerError(error)
    }
}

impl From<LiteralError> for TravelerErrorKind {
    fn from(error: LiteralError) -> Self {
        TravelerErrorKind::LiteralError(error)
    }
}

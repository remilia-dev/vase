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
    math::Sign,
    sync::Arc,
    util::{
        enum_with_properties,
        CachedString,
        FileId,
        Severity,
        SourceLocation,
    },
};

#[derive(Clone, Debug)]
pub struct TravelerError {
    pub state: TravelerState,
    pub kind: TravelerErrorKind,
}

enum_with_properties! {
    #[derive(Clone, Debug)]
    pub enum TravelerErrorKind {
        // == Others
        #[values(v0.severity(), v0.code())]
        LexerError(LexerError),
        #[values(v0.severity(), v0.code())]
        LiteralError(LiteralError),
        // == Internals
        #[values(Internal, "TI900")]
        Unimplemented(&'static str),
        #[values(Internal, "TI901")]
        Unreachable(&'static str),
        #[values(Internal, "TI902")]
        MissingIncludeId(FileId),
        // == Fatals
        #[values(Error, "TF490")]
        ErrorPreprocessor(Option<Arc<Box<str>>>),
        // == Errors
        #[values(Error, "TE300")]
        IfDefMissingId(bool),
        #[values(Error, "TE301")]
        IfDefExpectedId(bool, Token),
        #[values(Error, "TE310")]
        IfExpectedAtom(bool, Token),
        #[values(Error, "TE311")]
        IfExpectedOp(bool, Token),
        #[values(Error, "TE312")]
        IfDefinedNotDefinable(bool, Token),
        #[values(Error, "TE313")]
        IfDefinedExpectedRParen(bool, Token),
        #[values(Error, "TE314")]
        IfExpectedRParen(bool, Token),
        #[values(Error, "TE315")]
        IfTernaryExpectedColon(bool, Token),
        #[values(Error, "TE316")]
        IfDiv0(Sign, Box<BinaryExpr>),
        #[values(Error, "TE317")]
        IfReal(bool, Token),
        #[values(Error, "TE320")]
        DefineMissingId,
        #[values(Error, "TE331")]
        DefineExpectedId(Token),
        #[values(Error, "TE332")]
        DefineFuncEndBeforeEndOfArgs,
        #[values(Error, "TE333")]
        DefineFuncExpectedArg(Token),
        #[values(Error, "TE334")]
        DefineFuncExpectedSeparator(Token),
        #[values(Error, "TE335")]
        DefineFuncExpectedEndOfArgs(Token),
        #[values(Error, "TE340")]
        UndefMissingId,
        #[values(Error, "TE341")]
        UndefExpectedId(Token),
        #[values(Error, "TE350")]
        IncludePathMissing,
        #[values(Error, "TE351")]
        IncludeExpectedPath(Token),
        #[values(Error, "TE352")]
        IncludeNotFound(IncludeType, CachedString),
        #[values(Error, "TE360")]
        FuncInvokeMissingArgs(usize),
        #[values(Error, "TE361")]
        FuncInvokeExcessParameters(Vec<Token>),
        #[values(Error, "TE362")]
        FuncInvokePreprocessorInArgs(Token),
        #[values(Error, "TE363")]
        InnerFuncInvokeUnfinished,
        #[values(Error, "TE364")]
        StringifyExpectsId(Token),
        #[values(Error, "TE365")]
        StringifyNonParameter(Token),
        #[values(Error, "TE370")]
        InvalidJoin(Token, SourceLocation, Token),
        #[values(Error, "TE380")]
        StrayHash,
        #[values(Error, "TE381")]
        StrayHashHash,
        #[values(Error, "TE382")]
        StrayAtSign,
        #[values(Error, "TE383")]
        StrayBackslash,
        #[values(Error, "TE390")]
        UnknownPreprocessor(CachedString),
        // == Warning
        #[values(Warning, "TW200")]
        ExtraTokensInIfDef(bool),
        #[values(Warning, "TW201")]
        ExtraTokensInElse,
        #[values(Warning, "TW202")]
        ExtraTokensInEndIf,
        #[values(Warning, "TW203")]
        ExtraTokensInUndef,
        #[values(Warning, "TW204")]
        ExtraTokensInInclude,
        #[values(Warning, "TW210")]
        CommaInIfCondition,
        #[values(Warning, "TW211")]
        OverflowInIfBinary(i64, i64, Box<BinaryExpr>),
        #[values(Warning, "TW212")]
        OverflowInIfNegation(i64, Box<PrefixExpr>),
        #[values(Warning, "TW213")]
        NegativeSignedToUnsigned(bool, i64, Box<BinaryExpr>),
        #[values(Warning, "TW214")]
        ShiftedToMuch(Sign, Sign, Box<BinaryExpr>),
        #[values(Warning, "TW280")]
        WarningPreprocessor(Option<Arc<Box<str>>>),
        #[values(Warning, "TW299")]
        UnsupportableLinePreprocessor,
    }

    impl TravelerErrorKind {
        #[property]
        pub fn severity(&self) -> Severity {
            use Severity::*;
        }
        #[property]
        pub fn code(&self) -> &'static str {}
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::{
            BlockExpr,
            DeclStmt,
            Expr,
            ScopeId,
        },
        TravelIndex,
        TravelRange,
    },
    util::{
        create_intos,
        CachedString,
    },
};

#[create_intos]
#[derive(Clone, Debug)]
pub enum Stmt {
    Expr(Expr),
    Break(BreakStmt),
    Continue(ContinueStmt),
    Case(CaseStmt),
    Return(ReturnStmt),
    Goto(GotoStmt),
    Block(BlockExpr),
    If(IfStmt),
    While(WhileStmt),
    Do(DoStmt),
    For(ForStmt),
    Switch(SwitchStmt),
    Decl(DeclStmt),
    Empty(TravelIndex),
}

impl Stmt {
    pub fn requires_semicolon(&self) -> bool {
        use Stmt::*;
        matches!(
            *self,
            Expr(..) | Break(..) | Continue(..) | Return(..) | Goto(..) | Do(..)
        )
    }
}

#[derive(Clone, Debug)]
pub struct BreakStmt {
    pub break_scope_id: Option<ScopeId>,
    pub break_index: TravelIndex,
}

#[derive(Clone, Debug)]
pub struct ContinueStmt {
    pub continue_scope_id: Option<ScopeId>,
    pub continue_index: TravelIndex,
}

#[derive(Clone, Debug)]
pub struct CaseStmt {
    pub range: TravelRange,
    pub case: Option<Box<Expr>>,
    pub stmt: Box<Stmt>,
    pub switch_scope: Option<ScopeId>,
}

#[derive(Clone, Debug)]
pub struct ReturnStmt {
    pub return_index: TravelIndex,
    pub expr: Option<Box<Expr>>,
}

#[derive(Clone, Debug)]
pub struct GotoStmt {
    pub range: TravelRange,
    pub label_scope_id: Option<ScopeId>,
    pub label: Option<CachedString>,
}

#[derive(Clone, Debug)]
pub struct IfStmt {
    pub range: TravelRange,
    pub condition: Box<Expr>,
    pub block: Box<Stmt>,
    pub else_: Option<Box<Stmt>>,
}

#[derive(Clone, Debug)]
pub struct WhileStmt {
    pub range: TravelRange,
    pub condition: Box<Expr>,
    pub block: Box<Stmt>,
}

#[derive(Clone, Debug)]
pub struct DoStmt {
    pub range: TravelRange,
    pub block: Box<Stmt>,
    pub condition: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct ForStmt {
    pub range: TravelRange,
    pub initial: Box<Stmt>,
    pub condition: Option<Box<Expr>>,
    pub increment: Option<Box<Expr>>,
    pub block: Box<Stmt>,
}

#[derive(Clone, Debug)]
pub struct SwitchStmt {
    pub range: TravelRange,
    pub value: Box<Expr>,
    pub block: Box<Stmt>,
}

pub struct LabeledStmt {
    pub range: TravelRange,
    pub name: CachedString,
    pub stmt: Box<Stmt>,
}

pub struct CasedStmt {
    pub range: TravelRange,
    pub switch_scope: Option<ScopeId>,
    pub expression: Box<Expr>,
    pub stmt: Box<Stmt>,
}

pub struct DefaultStmt {
    pub range: TravelRange,
    pub switch_scope: Option<ScopeId>,
    pub stmt: Box<Stmt>,
}

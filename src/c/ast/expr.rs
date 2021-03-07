// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::ast::{
        Associativity,
        BinaryOp,
        Literal,
        Precedence,
        PrefixOp,
    },
    util::{
        create_intos,
        SourceLoc,
    },
};

#[create_intos]
#[derive(Clone, Debug)]
pub enum Expr {
    Literal(Literal),
    Parens(ParenExpr),
    Prefix(PrefixExpr),
    Binary(BinaryExpr),
    Ternary(TernaryExpr),
    Assignment(AssignmentExpr),
}

impl Expr {
    pub fn precedence(&self) -> Precedence {
        use Expr::*;
        match *self {
            Literal(..) | Parens(..) => Precedence::Atoms,
            Binary(ref expr) => expr.op.precedence(),
            Prefix(..) => Precedence::Prefixes,
            Ternary(..) => Precedence::Ternary,
            Assignment(..) => Precedence::Assignment,
        }
    }

    pub fn add_op<T>(mut self: Box<Self>, precedence: Precedence, create: T) -> Box<Self>
    where T: FnOnce(Box<Expr>) -> Box<Expr> {
        match (self.precedence(), precedence) {
            (s, o) if s < o => create(self),
            (s, o) if s > o => {
                self.take_right(precedence, create);
                self
            },
            _ => match precedence.associativity() {
                Associativity::LeftToRight => create(self),
                Associativity::RightToLeft => {
                    self.take_right(precedence, create);
                    self
                },
                Associativity::None => {
                    // TODO: Produce error?
                    panic!("TODO:");
                },
            },
        }
    }

    fn take_right<T>(&mut self, precedence: Precedence, create: T)
    where T: FnOnce(Box<Expr>) -> Box<Expr> {
        use replace_with::replace_with_or_abort as replace_or_abort;
        let replace_with = |rhs: Box<Expr>| rhs.add_op(precedence, create);
        match *self {
            Self::Literal(..) => panic!("Can't take right on a literal. It makes no sense!"),
            Self::Parens(..) => panic!("Can't take right on parenthesis. It makes no sense!"),
            Self::Prefix(ref mut expr) => replace_or_abort(&mut expr.expr, replace_with),
            Self::Binary(ref mut expr) => replace_or_abort(&mut expr.rhs, replace_with),
            Self::Ternary(ref mut expr) => replace_or_abort(&mut expr.if_false, replace_with),
            Self::Assignment(ref mut expr) => replace_or_abort(&mut expr.value, replace_with),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParenExpr {
    pub lparen_loc: Option<SourceLoc>,
    pub expr: Box<Expr>,
    pub rparen_loc: Option<SourceLoc>,
}

#[derive(Clone, Debug)]
pub struct PrefixExpr {
    pub op: PrefixOp,
    pub op_loc: SourceLoc,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub lhs: Box<Expr>,
    pub op: BinaryOp,
    pub op_loc: SourceLoc,
    pub rhs: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct TernaryExpr {
    pub condition: Box<Expr>,
    pub qmark_loc: SourceLoc,
    pub if_true: Box<Expr>,
    pub colon_loc: SourceLoc,
    pub if_false: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct AssignmentExpr {
    pub to: Box<Expr>,
    pub op: Option<BinaryOp>,
    pub op_loc: SourceLoc,
    pub value: Box<Expr>,
}

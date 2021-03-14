// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::{
        Associativity,
        BinaryOp,
            Number,
        Precedence,
        PrefixOp,
    },
        TravelIndex,
        TravelRange,
    },
    util::create_intos,
};

#[create_intos]
#[derive(Clone, Debug)]
pub enum Expr {
    Number(Number),
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
            Number(..) | Parens(..) => Precedence::Atoms,
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
            Self::Number(..) => panic!("Can't take right on a number. It makes no sense!"),
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
    /// The range of traveler indexes this expression covers.
    ///
    /// If parsed without error, the start index should be a ( token
    /// and the end index should be a ) token.
    pub range: TravelRange,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct PrefixExpr {
    /// The range of traveler indexes this expression covers.
    /// The start index should be the operator token.
    pub range: TravelRange,
    pub op: PrefixOp,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub lhs: Box<Expr>,
    pub op: BinaryOp,
    pub op_index: TravelIndex,
    pub rhs: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct TernaryExpr {
    pub condition: Box<Expr>,
    pub qmark_index: TravelIndex,
    pub if_true: Box<Expr>,
    pub colon_index: TravelIndex,
    pub if_false: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct AssignmentExpr {
    pub to: Box<Expr>,
    pub op: Option<BinaryOp>,
    pub op_index: TravelIndex,
    pub value: Box<Expr>,
}

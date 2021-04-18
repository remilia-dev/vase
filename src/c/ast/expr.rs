// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use smallvec::SmallVec;

use crate::{
    c::{
        ast::*,
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
pub enum Expr {
    // Atoms:
    DeclRef(DeclRefExpr),
    Number(Number),
    String(StringLiteral),
    Block(BlockExpr),
    Parens(ParenExpr),
    Init(InitExpr),
    // Suffixes:
    Suffix(SuffixExpr),
    Access(AccessExpr),
    Array(ArrayExpr),
    Call(CallExpr),
    // Other:
    Type(TypeExpr),
    Prefix(PrefixExpr),
    Cast(CastExpr),
    Binary(BinaryExpr),
    Ternary(TernaryExpr),
    Assign(AssignExpr),
}

impl Expr {
    pub fn precedence(&self) -> Precedence {
        use Expr::*;
        match *self {
            DeclRef(..) | Number(..) | String(..) | Block(..) | Parens(..) | Init(..) => {
                Precedence::Atoms
            },
            Suffix(..) | Access(..) | Array(..) | Call(..) => Precedence::Suffixes,
            Type(ref expr) => expr.precedence(),
            Prefix(..) => Precedence::Prefixes,
            Cast(..) => Precedence::Prefixes,
            Binary(ref expr) => expr.op.precedence(),
            Ternary(..) => Precedence::Ternary,
            Assign(..) => Precedence::Assignment,
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
        use Expr::*;
        let right_item = match *self {
            DeclRef(..) | Number(..) | String(..) | Block(..) | Parens(..) | Init(..) => panic!(
                "Can't take right on an atom (identifier/number/string/block/paren) expression."
            ),
            Suffix(..) | Access(..) | Array(..) | Call(..) => {
                panic!("Can't take right on a suffix expression.");
            },
            Type(ref mut expr) => expr.get_right(),
            Prefix(ref mut expr) => &mut expr.expr,
            Cast(ref mut expr) => &mut expr.expr,
            Binary(ref mut expr) => &mut expr.rhs,
            Ternary(ref mut expr) => &mut expr.if_false,
            Assign(ref mut expr) => &mut expr.value,
        };
        replace_or_abort(right_item, replace_with)
    }
}

#[derive(Clone, Debug)]
pub struct BlockExpr {
    /// The range of traveler indexes this expression covers.
    ///
    /// If parsed without error, the start index should be a LBrace token
    /// and the end index should be a RBrace token.
    pub range: TravelRange,
    pub scope_id: ScopeId,
}

#[derive(Clone, Debug)]
pub struct ParenExpr {
    /// The range of traveler indexes this expression covers.
    ///
    /// If parsed without error, the start index should be a LParen token
    /// and the end index should be a RParen token.
    pub range: TravelRange,
    pub expr: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct InitExpr {
    pub range: TravelRange,
    pub values: Vec<InitMember>,
}

#[derive(Clone, Debug)]
pub enum InitMember {
    Unnamed(Expr),
    Named(Id, Expr),
    Array(SmallVec<[Expr; 1]>, Expr),
    SubInitializer(InitExpr),
}

#[derive(Clone, Debug)]
pub struct SuffixExpr {
    pub expr: Box<Expr>,
    pub op: SuffixOp,
    pub op_index: TravelIndex,
}

#[derive(Clone, Debug)]
pub struct AccessExpr {
    // The range of the access expression.
    //
    // The start index should be the Dot/Arrow token
    pub range: TravelRange,
    pub expr: Box<Expr>,
    pub through_ptr: bool,
    pub member: CachedString,
}

#[derive(Clone, Debug)]
pub struct ArrayExpr {
    /// The range of traveler indexes this expression covers.
    ///
    /// If parsed without error, the start index should be the LBracket token
    /// and the last index the RBracket token.
    pub range: TravelRange,
    pub expr: Box<Expr>,
    pub offset: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    /// The range of traveler indexes this expression covers.
    ///
    /// If parsed without error, the start index should be the LParen token
    /// and the last index the RParen token.
    pub range: TravelRange,
    pub expr: Box<Expr>,
    pub args: Vec<Expr>,
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
pub struct TypeExpr {
    pub range: TravelRange,
    pub op: TypeOp,
    pub of: TypeOrExpr,
}

impl TypeExpr {
    fn precedence(&self) -> Precedence {
        if matches!(self.of, TypeOrExpr::Type(..)) {
            Precedence::Atoms
        } else {
            Precedence::Prefixes
        }
    }

    fn get_right(&mut self) -> &mut Box<Expr> {
        if let TypeOrExpr::Expr(ref mut expr) = self.of {
            expr
        } else {
            panic!("Can't take right on a type atom expression (sizeof/_Alignof).")
        }
    }
}

#[create_intos]
#[derive(Clone, Debug)]
pub enum TypeOrExpr {
    Type(Type),
    Expr(Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct CastExpr {
    pub range: TravelRange,
    pub to: Type,
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
pub struct AssignExpr {
    pub to: Box<Expr>,
    pub op: AssignOp,
    pub op_index: TravelIndex,
    pub value: Box<Expr>,
}

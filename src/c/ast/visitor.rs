// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::ast::*,
    error::MayUnwind,
};

pub trait ExprVisitor {
    fn on_expr(&mut self, expr: &mut Expr) -> MayUnwind<()> {
        self.visit_expr(expr)
    }
    fn visit_expr(&mut self, expr: &mut Expr) -> MayUnwind<()> {
        match *expr {
            Expr::DeclRef(ref _ref_) => todo!(),
            Expr::String(..) => todo!(),
            Expr::Number(ref mut lit) => self.on_number(lit),
            Expr::Parens(ref mut expr) => self.on_parens(expr),
            Expr::Init(_) => todo!(),  // TODO: ?
            Expr::Block(_) => todo!(), // TODO: DO
            Expr::Suffix(_) => todo!(),
            Expr::Access(_) => todo!(), // TODO: ?
            Expr::Array(_) => todo!(),
            Expr::Call(_) => todo!(),
            Expr::Type(..) => todo!(),
            Expr::Cast(ref mut expr) => self.on_cast(expr),
            Expr::Prefix(ref mut expr) => self.on_prefix(expr),
            Expr::Binary(ref mut expr) => self.on_binary(expr),
            Expr::Ternary(ref mut expr) => self.on_ternary(expr),
            Expr::Assign(ref mut expr) => self.on_assign(expr),
        }
    }

    fn on_number(&mut self, _lit: &mut Number) -> MayUnwind<()> {
        Ok(())
    }

    fn on_parens(&mut self, expr: &mut ParenExpr) -> MayUnwind<()> {
        self.visit_parens(expr)
    }
    fn visit_parens(&mut self, expr: &mut ParenExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.expr)
    }

    fn on_cast(&mut self, expr: &mut CastExpr) -> MayUnwind<()> {
        self.visit_cast(expr)
    }
    fn visit_cast(&mut self, expr: &mut CastExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.expr)
    }

    fn on_prefix(&mut self, expr: &mut PrefixExpr) -> MayUnwind<()> {
        self.visit_prefix(expr)
    }
    fn visit_prefix(&mut self, expr: &mut PrefixExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.expr)
    }

    fn on_binary(&mut self, expr: &mut BinaryExpr) -> MayUnwind<()> {
        self.visit_binary(expr)
    }
    fn visit_binary(&mut self, expr: &mut BinaryExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.lhs)?;
        self.on_expr(&mut expr.rhs)
    }

    fn on_ternary(&mut self, expr: &mut TernaryExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.condition)?;
        self.on_expr(&mut expr.if_true)?;
        self.on_expr(&mut expr.if_false)
    }

    fn on_assign(&mut self, expr: &mut AssignExpr) -> MayUnwind<()> {
        self.visit_assign(expr)
    }
    fn visit_assign(&mut self, expr: &mut AssignExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.value)?;
        self.on_expr(&mut expr.to)
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::*,
        Token,
    },
    error::{
        MayUnwind,
        Unwind,
    },
    math::{
        Integer,
        Sign,
    },
    util::Conversions,
};

type Error = crate::c::TravelerErrorKind;
pub trait OnError = FnMut(Error) -> MayUnwind<()>;

pub struct IfEvaluator<E: OnError> {
    accum: Option<Sign>,
    on_error: E,
    if_token: Token,
}

impl<E: OnError> ExprVisitor for IfEvaluator<E> {
    fn on_number(&mut self, lit: &mut Number) -> MayUnwind<()> {
        // If this is the first literal we've encountered, make the accumulator it.
        if self.accum.is_none() {
            self.accum = match lit.kind {
                NumberKind::I64(i) => Some(i.into()),
                NumberKind::U64(u) => Some(u.into()),
                _ => {
                    (self.on_error)(Error::Unreachable(
                        "Only I64 and U64 literals should appear in an #if/#elif tree.",
                    ))?;
                    return Err(Unwind::Fatal);
                },
            };
        }
        Ok(())
    }

    fn on_prefix(&mut self, expr: &mut PrefixExpr) -> MayUnwind<()> {
        self.visit_prefix(expr)?;
        use PrefixOp::*;
        let accum = self.accum.take().unwrap();
        self.accum = match expr.op {
            Posate => accum,
            Negate => match accum {
                Sign::Signed(i) => {
                    let (neg_i, overflowed) = i.overflowing_neg();
                    if overflowed {
                        let error = Error::OverflowInIfNegation(i, expr.clone().into());
                        (self.on_error)(error)?;
                    }
                    neg_i.into()
                },
                Sign::Unsigned(u) => (-(u as i64) as u64).into(),
            },
            BitNot => !accum,
            LogicalNot => {
                if accum.is_zero() {
                    1i64.into()
                } else {
                    0i64.into()
                }
            },
            _ => {
                (self.on_error)(Error::Unreachable(
                    "Only +, -, ~, and ! unary operators should occur in #if/#elif",
                ))?;
                return Err(Unwind::Fatal);
            },
        }
        .into();
        Ok(())
    }

    fn on_binary(&mut self, expr: &mut BinaryExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.rhs)?;
        let rhs = self.accum.take().unwrap();
        self.on_expr(&mut expr.lhs)?;
        let lhs = self.accum.take().unwrap();
        use BinaryOp::*;
        let lit = if rhs.is_unsigned() || lhs.is_unsigned() {
            let lhs = self.as_unsigned(lhs, false, &*expr)?;
            let rhs = self.as_unsigned(rhs, true, expr)?;
            match expr.op {
                Multiplication => lhs.wrapping_mul(rhs),
                Divide => self.may_div_0(lhs, lhs.checked_div(rhs), expr)?,
                Modulo => self.may_div_0(lhs, lhs.checked_rem(rhs), expr)?,
                Addition => lhs.wrapping_add(rhs),
                Subtraction => lhs.wrapping_sub(rhs),
                LShift => self.shift(false, lhs, rhs, expr)?,
                RShift => self.shift(true, lhs, rhs, expr)?,
                LessThan => (lhs < rhs) as u64,
                LessThanOrEqual => (lhs <= rhs) as u64,
                GreaterThan => (lhs > rhs) as u64,
                GreaterThanOrEqual => (lhs >= rhs) as u64,
                Equals => (lhs == rhs) as u64,
                NotEquals => (lhs != rhs) as u64,
                BitAnd => lhs & rhs,
                BitXor => lhs ^ rhs,
                BitOr => lhs | rhs,
                LogicalOr => (lhs != 0 || rhs != 0) as u64,
                LogicalAnd => (lhs != 0 && rhs != 0) as u64,
            }
            .into()
        } else {
            let (lhs, _) = lhs.wrapped_signed();
            let (rhs, _) = rhs.wrapped_signed();
            match expr.op {
                Multiplication => self.may_overflow(lhs.overflowing_mul(rhs), lhs, rhs, expr)?,
                Divide => self.may_div_0(lhs, lhs.checked_div(rhs), expr)?,
                Modulo => self.may_div_0(lhs, lhs.checked_rem(rhs), expr)?,
                Addition => self.may_overflow(lhs.overflowing_add(rhs), lhs, rhs, expr)?,
                Subtraction => self.may_overflow(lhs.overflowing_sub(rhs), lhs, rhs, expr)?,
                LShift => self.shift(false, lhs, rhs, expr)?,
                RShift => self.shift(true, lhs, rhs, expr)?,
                LessThan => (lhs < rhs) as i64,
                LessThanOrEqual => (lhs <= rhs) as i64,
                GreaterThan => (lhs > rhs) as i64,
                GreaterThanOrEqual => (lhs >= rhs) as i64,
                Equals => (lhs == rhs) as i64,
                NotEquals => (lhs != rhs) as i64,
                BitAnd => lhs & rhs,
                BitXor => lhs ^ rhs,
                BitOr => lhs | rhs,
                LogicalOr => (lhs != 0 || rhs != 0) as i64,
                LogicalAnd => (lhs != 0 && rhs != 0) as i64,
            }
            .into()
        };
        self.accum = Some(lit);
        Ok(())
    }

    fn on_ternary(&mut self, expr: &mut TernaryExpr) -> MayUnwind<()> {
        self.on_expr(&mut expr.condition)?;
        match self.accum.take() {
            Some(lit) if !lit.is_zero() => self.visit_expr(&mut expr.if_true),
            _ => self.visit_expr(&mut expr.if_false),
        }
    }

    fn on_assign(&mut self, _: &mut AssignExpr) -> MayUnwind<()> {
        (self.on_error)(Error::Unreachable(
            "Assignment operators should not occur in a #if/#elif condition",
        ))
    }
}

impl<E: OnError> IfEvaluator<E> {
    pub fn calc(e: &mut Expr, if_token: Token, on_error: E) -> MayUnwind<bool> {
        let mut visitor = IfEvaluator { accum: None, if_token, on_error };
        visitor.on_expr(e)?;
        Ok(visitor.accum.map_or(false, |v| !v.is_zero()))
    }

    fn as_unsigned(&mut self, s: Sign, rhs: bool, expr: &BinaryExpr) -> MayUnwind<u64> {
        match s.try_into() {
            Ok(u) => Ok(u),
            Err(i) => {
                (self.on_error)(Error::NegativeSignedToUnsigned(
                    rhs,
                    i,
                    expr.clone().into(),
                ))?;
                Ok(i as u64)
            },
        }
    }

    fn may_overflow(
        &mut self,
        v: (i64, bool),
        lhs: i64,
        rhs: i64,
        expr: &BinaryExpr,
    ) -> MayUnwind<i64> {
        if v.1 {
            let error = Error::OverflowInIfBinary(lhs, rhs, expr.clone().into());
            (self.on_error)(error)?;
        }
        Ok(v.0)
    }

    fn may_div_0<I>(&mut self, lhs: I, v: Option<I>, expr: &BinaryExpr) -> MayUnwind<I>
    where I: Into<Sign> {
        if let Some(v) = v {
            Ok(v)
        } else {
            (self.on_error)(Error::IfDiv0(
                self.if_token.clone(),
                lhs.into(),
                expr.clone().into(),
            ))?;
            Err(Unwind::Block)
        }
    }

    fn shift<I>(&mut self, shr: bool, lhs: I, rhs: I, expr: &BinaryExpr) -> MayUnwind<I>
    where I: Integer + Into<Sign> {
        let shifted = if shr {
            lhs.checked_shr(rhs)
        } else {
            lhs.checked_shl(rhs)
        };
        match shifted {
            Some(v) => Ok(v),
            None => {
                (self.on_error)(Error::ShiftedToMuch(
                    lhs.into(),
                    rhs.into(),
                    expr.clone().into(),
                ))?;
                if shr && lhs < I::from(0) {
                    Ok(!I::from(0))
                } else {
                    Ok(I::from(0))
                }
            },
        }
    }
}

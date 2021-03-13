// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::convert::TryInto;

use crate::{
    c::{
        ast::{
            BinaryExpr,
            BinaryOp,
            Expr,
            Literal,
            LiteralError,
            LiteralKind,
            ParenExpr,
            Precedence,
            PrefixExpr,
            PrefixOp,
            TernaryExpr,
        },
        StringEnc,
        Token,
        TokenKind::*,
        Traveler,
        TravelerError,
    },
    error::{
        MayUnwind,
        Unwind,
    },
    util::{
        CachedString,
        SourceLoc,
    },
};

type Error = crate::c::TravelerErrorKind;

pub struct IfParser<'a, OnError>
where OnError: FnMut(TravelerError) -> bool
{
    traveler: &'a mut Traveler<OnError>,
    if_token: &'a Token,
    defined_id: usize,
}

impl<'a, OnError> IfParser<'a, OnError>
where OnError: FnMut(TravelerError) -> bool
{
    pub fn create_and_parse(
        traveler: &'a mut Traveler<OnError>,
        if_token: &'a Token,
    ) -> MayUnwind<Box<Expr>> {
        let defined_id = traveler.env.cache().get_or_cache("defined").uniq_id();
        Self { traveler, if_token, defined_id }.parse_expression()
    }

    fn parse_expression(&mut self) -> MayUnwind<Box<Expr>> {
        let mut expression = self.parse_atom()?;

        loop {
            let head = self.head();
            expression = match *head.kind() {
                ref op if op.is_binary_op() => {
                    let op_loc = head.loc();
                    let op: BinaryOp = op.try_into().unwrap();
                    self.move_forward()?;
                    let rhs = self.parse_atom()?;
                    expression.add_op(op.precedence(), |lhs| {
                        Expr::Binary(BinaryExpr { lhs, op, op_loc, rhs }).into()
                    })
                },
                QMark => {
                    let qmark_loc = head.loc();
                    self.parse_ternary(expression, qmark_loc)?
                },
                Comma => {
                    self.report_error(Error::CommaInIfCondition)?;
                    self.move_forward()?;
                    self.parse_expression()?
                },
                RParen | Colon | PreEnd => break,
                _ => {
                    let error = Error::IfExpectedOp(self.if_token.clone(), head.clone());
                    self.report_error(error)?;
                    return Err(Unwind::Block);
                },
            }
        }

        Ok(expression)
    }

    fn parse_ternary(
        &mut self,
        expression: Box<Expr>,
        qmark_loc: SourceLoc,
    ) -> MayUnwind<Box<Expr>> {
        // Move past the ?
        self.move_forward()?;
        let if_true = self.parse_expression()?;

        let maybe_colon = self.head();
        let colon_loc = if matches!(*maybe_colon.kind(), Colon) {
            maybe_colon.loc()
        } else {
            let error = Error::IfTernaryExpectedColon(self.if_token.clone(), self.clone_head());
            self.report_error(error)?;
            return Err(Unwind::Block);
        };
        // Move past the :
        self.move_forward()?;

        let if_false = self.parse_expression()?;

        Ok(expression.add_op(Precedence::Ternary, |condition| {
            Expr::Ternary(TernaryExpr {
                condition,
                qmark_loc,
                if_true,
                colon_loc,
                if_false,
            })
            .into()
        }))
    }

    fn parse_atom(&mut self) -> MayUnwind<Box<Expr>> {
        let head = self.head();
        match *head.kind() {
            // 'defined macro_id' or 'defined(macro_id)'
            Identifier(ref id) if id.uniq_id() == self.defined_id => {
                let loc = head.loc();
                self.parse_defined(loc)
            },
            // Undefined identifiers are replaced with 0s
            Identifier(..) => {
                let loc = head.loc();
                self.move_forward()?;
                Ok(Box::new(Literal { loc, kind: 0i64.into() }.into()))
            },
            Number(ref digits) => {
                let digits = digits.clone();
                let loc = head.loc();
                self.parse_number(loc, digits)
            },
            Plus | Minus | Tilde | Bang => {
                let op: PrefixOp = head.kind().try_into().unwrap();
                let op_loc = head.loc();
                self.move_forward()?;
                let expr = self.parse_atom()?;
                Ok(Box::new(PrefixExpr { op, op_loc, expr }.into()))
            },
            LParen { .. } => {
                let lparen_loc = Some(head.loc());
                self.parse_parens(lparen_loc)
            },
            String {
                is_char: true,
                ref str_data,
                encoding,
                ..
            } => {
                let str_data = str_data.clone();
                let loc = head.loc();
                self.parse_char(loc, &*str_data, encoding)
            },
            PreEnd => {
                let loc = head.loc();
                let error = Error::IfExpectedAtom(self.if_token.clone(), head.clone());
                self.report_error(error)?;
                Ok(Box::new(Literal { loc, kind: 0i64.into() }.into()))
            },
            _ => {
                let error = Error::IfExpectedAtom(self.if_token.clone(), head.clone());
                self.report_error(error)?;
                // Cascade up to the if condition. We can't parse the expression.
                Err(Unwind::Block)
            },
        }
    }

    fn parse_defined(&mut self, loc: SourceLoc) -> MayUnwind<Box<Expr>> {
        let (head, has_parens) = match self.move_frame_forward() {
            token if matches!(token.kind(), &LParen) => (self.move_frame_forward(), true),
            token => (token, false),
        };

        let id = match *head.kind() {
            ref kind if kind.is_definable() => kind.get_definable_id(),
            RParen if has_parens => {
                let error = Error::IfDefinedNotDefinable(
                    self.if_token.clone(),
                    has_parens,
                    self.clone_head(),
                );
                self.report_error(error)?;
                self.move_forward()?;
                return Ok(Box::new(Literal { loc, kind: 0i64.into() }.into()));
            },
            PreEnd => {
                let loc = head.loc();
                let error = Error::IfDefinedNotDefinable(
                    self.if_token.clone(),
                    has_parens,
                    self.clone_head(),
                );
                self.report_error(error)?;
                return Ok(Box::new(Literal { loc, kind: 0i64.into() }.into()));
            },
            _ => {
                let error = Error::IfDefinedNotDefinable(
                    self.if_token.clone(),
                    has_parens,
                    self.clone_head(),
                );
                self.report_error(error)?;
                0 // A unique id of 0 will never show up
            },
        };

        match *self.move_frame_forward().kind() {
            RParen if has_parens => {
                self.move_forward()?;
            },
            _ if has_parens => {
                let error =
                    Error::IfDefinedExpectedRParen(self.if_token.clone(), self.clone_head());
                self.report_error(error)?;
            },
            _ => {},
        }

        let value = self.traveler.frames.has_macro(id) as i64;
        Ok(Box::new(Literal { loc, kind: value.into() }.into()))
    }

    fn parse_parens(&mut self, lparen_loc: Option<SourceLoc>) -> MayUnwind<Box<Expr>> {
        self.move_forward()?;
        let expr = self.parse_expression()?;

        let head = self.head();
        let rparen_loc = match *head.kind() {
            RParen => {
                let loc = head.loc();
                self.move_forward()?;
                Some(loc)
            },
            _ => {
                let error = Error::IfExpectedRParen(self.if_token.clone(), self.clone_head());
                self.report_error(error)?;
                None
            },
        };
        Ok(Box::new(
            ParenExpr { lparen_loc, expr, rparen_loc }.into(),
        ))
    }

    fn parse_number(&mut self, loc: SourceLoc, digits: CachedString) -> MayUnwind<Box<Expr>> {
        let mut kind = LiteralKind::from_number(digits.as_ref(), |err: LiteralError| {
            self.report_error(err.into())
        })?;
        if kind.is_real() {
            let error = Error::IfReal(self.if_token.clone(), self.clone_head());
            self.report_error(error)?;
        }
        kind = match kind {
            LiteralKind::I32(i) => (i as i64).into(),
            LiteralKind::U32(u) => (u as u64).into(),
            LiteralKind::F32(f) => (f as i64).into(),
            LiteralKind::F64(f) => (f as i64).into(),
            l => l,
        };
        self.move_forward()?;
        Ok(Box::new(Literal { loc, kind }.into()))
    }

    fn parse_char(&mut self, loc: SourceLoc, chars: &str, enc: StringEnc) -> MayUnwind<Box<Expr>> {
        let mut kind = LiteralKind::from_character(chars, enc, |err: LiteralError| {
            self.report_error(err.into())
        })?;
        kind = match kind {
            // Only the I32 should occur
            LiteralKind::I32(i) => (i as i64).into(),
            LiteralKind::U32(u) => (u as u64).into(),
            LiteralKind::F32(f) => (f as i64).into(),
            LiteralKind::F64(f) => (f as i64).into(),
            l => l,
        };
        self.move_forward()?;
        Ok(Box::new(Literal { loc, kind }.into()))
    }

    fn move_frame_forward(&mut self) -> &Token {
        self.traveler.frames.move_forward()
    }

    fn move_forward(&mut self) -> MayUnwind<&Token> {
        self.traveler.move_forward()
    }

    fn head(&self) -> &Token {
        self.traveler.head()
    }

    fn clone_head(&self) -> Token {
        self.traveler.head().clone()
    }

    fn report_error(&mut self, error: Error) -> MayUnwind<()> {
        self.traveler.report_error(error)
    }
}

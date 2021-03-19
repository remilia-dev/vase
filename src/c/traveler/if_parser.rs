// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use crate::{
    c::{
        ast::{
            BinaryExpr,
            BinaryOp,
            Expr,
            Number,
            NumberError,
            NumberKind,
            ParenExpr,
            Precedence,
            PrefixExpr,
            PrefixOp,
            TernaryExpr,
        },
        StringEnc,
        Token,
        TokenKind::*,
        TravelIndex,
        Traveler,
        TravelerError,
    },
    error::{
        ErrorReceiver,
        MayUnwind,
        Unwind,
    },
    util::{
        CachedString,
        Conversions,
    },
};

type Error = crate::c::TravelerErrorKind;

pub struct IfParser<'a, 'b, E: ErrorReceiver<TravelerError>> {
    traveler: &'a mut Traveler<'b, E>,
    if_token: &'a Token,
    defined_id: usize,
}

impl<'a, 'b, E: ErrorReceiver<TravelerError>> IfParser<'a, 'b, E> {
    pub fn create_and_parse(
        traveler: &'a mut Traveler<'b, E>,
        if_token: &'a Token,
    ) -> MayUnwind<Box<Expr>> {
        let defined_id = traveler.env.cache().get_or_cache("defined").uniq_id();
        Self { traveler, if_token, defined_id }.parse_expression()
    }

    fn parse_expression(&mut self) -> MayUnwind<Box<Expr>> {
        let mut expr = self.parse_atom()?;

        loop {
            let head = self.head();
            if let Ok(op) = head.kind().try_into::<BinaryOp>() {
                let op_index = self.traveler.index();
                self.move_forward()?;
                let rhs = self.parse_atom()?;
                expr = expr.add_op(op.precedence(), |lhs| {
                    Expr::Binary(BinaryExpr { lhs, op, op_index, rhs }).into()
                });
            } else {
                expr = match *head.kind() {
                    QMark => self.parse_ternary(expr)?,
                    Comma => {
                        self.report_error(Error::CommaInIfCondition)?;
                        self.move_forward()?;
                        self.parse_expression()?
                    },
                    Colon | RParen | PreEnd => break,
                    _ => {
                        let error = Error::IfExpectedOp(self.if_token.clone(), head.clone());
                        self.report_error(error)?;
                        return Err(Unwind::Block);
                    },
                };
            }
        }

        Ok(expr)
    }

    fn parse_ternary(&mut self, expr: Box<Expr>) -> MayUnwind<Box<Expr>> {
        let qmark_index = self.traveler.index();
        // Move past the ?
        self.move_forward()?;
        let if_true = self.parse_expression()?;

        let maybe_colon = self.head();
        let colon_index = if matches!(*maybe_colon.kind(), Colon) {
            self.traveler.index()
        } else {
            let error = Error::IfTernaryExpectedColon(self.if_token.clone(), self.clone_head());
            self.report_error(error)?;
            return Err(Unwind::Block);
        };
        // Move past the :
        self.move_forward()?;

        let if_false = self.parse_expression()?;

        Ok(expr.add_op(Precedence::Ternary, |condition| {
            let new_expr = TernaryExpr {
                condition,
                qmark_index,
                if_true,
                colon_index,
                if_false,
            };
            Box::new(new_expr.into())
        }))
    }

    fn parse_atom(&mut self) -> MayUnwind<Box<Expr>> {
        let head = self.head();
        match *head.kind() {
            // 'defined macro_id' or 'defined(macro_id)'
            Identifier(ref id) if id.uniq_id() == self.defined_id => {
                let index = self.traveler.index();
                self.parse_defined(index)
            },
            // Undefined identifiers are replaced with 0s
            Identifier(..) => {
                let index = self.traveler.index();
                self.move_forward()?;
                Ok(Box::new(Number { kind: 0i64.into(), index }.into()))
            },
            Number(ref digits) => {
                let digits = digits.clone();
                let index = self.traveler.index();
                self.parse_number(index, digits)
            },
            Plus | Minus | Tilde | Bang => {
                let op: PrefixOp = head.kind().try_into().unwrap();
                let op_index = self.traveler.index();
                self.move_forward()?;
                let expr = self.parse_atom()?;
                let range = op_index..self.traveler.index();
                Ok(Box::new(PrefixExpr { range, op, expr }.into()))
            },
            LParen { .. } => {
                let lparen_index = self.traveler.index();
                self.parse_parens(lparen_index)
            },
            String {
                is_char: true,
                ref str_data,
                encoding,
                ..
            } => {
                let str_data = str_data.clone();
                let index = self.traveler.index();
                self.parse_char(index, &*str_data, encoding)
            },
            PreEnd => {
                let index = self.traveler.index();
                let error = Error::IfExpectedAtom(self.if_token.clone(), head.clone());
                self.report_error(error)?;
                Ok(Box::new(Number { index, kind: 0i64.into() }.into()))
            },
            _ => {
                let error = Error::IfExpectedAtom(self.if_token.clone(), head.clone());
                self.report_error(error)?;
                // Cascade up to the if condition. We can't parse the expression.
                Err(Unwind::Block)
            },
        }
    }

    fn parse_defined(&mut self, index: TravelIndex) -> MayUnwind<Box<Expr>> {
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
                return Ok(Box::new(Number { index, kind: 0i64.into() }.into()));
            },
            PreEnd => {
                let error = Error::IfDefinedNotDefinable(
                    self.if_token.clone(),
                    has_parens,
                    self.clone_head(),
                );
                self.report_error(error)?;
                return Ok(Box::new(Number { index, kind: 0i64.into() }.into()));
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
        Ok(Box::new(Number { index, kind: value.into() }.into()))
    }

    fn parse_parens(&mut self, lparen_index: TravelIndex) -> MayUnwind<Box<Expr>> {
        self.move_forward()?;
        let expr = self.parse_expression()?;

        let head = self.head();
        match *head.kind() {
            RParen => {
                self.move_forward()?;
            },
            _ => {
                let error = Error::IfExpectedRParen(self.if_token.clone(), self.clone_head());
                self.report_error(error)?;
            },
        };
        let range = lparen_index..self.traveler.index();
        Ok(Box::new(ParenExpr { range, expr }.into()))
    }

    fn parse_number(&mut self, index: TravelIndex, digits: CachedString) -> MayUnwind<Box<Expr>> {
        let mut kind = NumberKind::from_number(digits.as_ref(), &mut |err: NumberError| {
            self.traveler.report_error(err.into()).is_err()
        })?;
        if kind.is_real() {
            let error = Error::IfReal(self.if_token.clone(), self.clone_head());
            self.report_error(error)?;
        }
        kind = match kind {
            NumberKind::I32(i) => (i as i64).into(),
            NumberKind::U32(u) => (u as u64).into(),
            NumberKind::F32(f) => (f as i64).into(),
            NumberKind::F64(f) => (f as i64).into(),
            l => l,
        };
        self.move_forward()?;
        Ok(Box::new(Number { kind, index }.into()))
    }

    fn parse_char(
        &mut self,
        index: TravelIndex,
        chars: &str,
        enc: StringEnc,
    ) -> MayUnwind<Box<Expr>> {
        let mut kind = NumberKind::from_character(chars, enc, &mut |err: NumberError| {
            self.traveler.report_error(err.into()).is_err()
        })?;
        kind = match kind {
            // Only the I32 should occur
            NumberKind::I32(i) => (i as i64).into(),
            NumberKind::U32(u) => (u as u64).into(),
            NumberKind::F32(f) => (f as i64).into(),
            NumberKind::F64(f) => (f as i64).into(),
            l => l,
        };
        self.move_forward()?;
        Ok(Box::new(Number { kind, index }.into()))
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

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::cell::RefCell;

use smallvec::SmallVec;

use crate::{
    c::{
        ast::*,
        CompileEnv,
        FileTokens,
        Keyword,
        ParseError,
        ParseErrorKind,
        StringEnc,
        TokenKind,
        TravelIndex,
        Traveler,
        TravelerError,
    },
    error::{
        ErrorReceiver,
        MayUnwind,
        Unwind,
    },
    sync::Arc,
    util::Conversions,
};

type Error = ParseErrorKind;

pub struct Parser<'a, E: 'a + ErrorReceiver<ParseError>> {
    traveler: Traveler<'a, Box<dyn 'a + FnMut(TravelerError) -> bool>>,
    errors: Arc<RefCell<E>>,
}

impl<'a, E: ErrorReceiver<ParseError>> Parser<'a, E> {
    pub fn new(env: &'a CompileEnv, errors: E) -> Self {
        let shared_errors = Arc::new(RefCell::new(errors));
        let mut traveler_errors = shared_errors.clone();
        let travel_error_receiver =
            Box::new(move |error: TravelerError| traveler_errors.report(error.into()).is_err());
        Parser {
            traveler: Traveler::new(env, travel_error_receiver),
            errors: shared_errors,
        }
    }

    pub fn parse(&mut self, tokens: Arc<FileTokens>) -> MayUnwind<SourceFile> {
        ParseState::create_and_parse(self, tokens)
    }
}

struct ParseState<'a, 'b, E: 'b + ErrorReceiver<ParseError>> {
    traveler: &'a mut Traveler<'b, Box<dyn 'b + FnMut(TravelerError) -> bool>>,
    errors: &'a mut Arc<RefCell<E>>,
    file: SourceFile,
}

impl<'a, 'b, E: 'b + ErrorReceiver<ParseError>> ParseState<'a, 'b, E> {
    fn create_and_parse(
        parser: &'a mut Parser<'b, E>,
        tokens: Arc<FileTokens>,
    ) -> MayUnwind<SourceFile> {
        let mut parser = Self {
            traveler: &mut parser.traveler,
            errors: &mut parser.errors,
            file: SourceFile::new(tokens.file_id(), tokens.path().clone()),
        };
        parser.traveler.load_start(tokens)?;
        parser.file()?;
        Ok(parser.file)
    }

    fn file(&mut self) -> MayUnwind<()> {
        let scope_id = 0.into();
        loop {
            let stmt = match *self.traveler.head().kind() {
                TokenKind::Keyword(keyword) if keyword == Keyword::StaticAssert => {
                    // TODO: Parse static assert
                    todo!("_Static_assert parsing")
                },
                TokenKind::Semicolon => {
                    let stmt = Stmt::Empty(self.traveler.index());
                    // TODO: Warn on empty statement.
                    self.traveler.move_forward()?;
                    stmt
                },
                TokenKind::Eof => break,
                _ => self.decl_stmt(scope_id)?.into(),
            };

            self.file.get_scope_mut(scope_id).stmts.push(stmt);
        }
        Ok(())
    }

    fn decls(&mut self, scope_id: ScopeId, local: bool) -> MayUnwind<SmallVec<[Decl; 1]>> {
        let mut decls = SmallVec::new();

        let mut type_ = self.type_base(scope_id, local)?;
        loop {
            type_ = self.type_name(type_, scope_id)?;

            let postfix = match *self.traveler.head().kind() {
                TokenKind::Colon => {
                    self.traveler.move_forward()?;
                    let expr = self.expr(scope_id, false)?;
                    DeclPostfix::Bitfield(expr)
                },
                TokenKind::Equal => {
                    self.traveler.move_forward()?;
                    let expr = self.expr(scope_id, false)?;
                    DeclPostfix::Initializer(expr)
                },
                TokenKind::LBrace { .. } => {
                    if let Some(func_scope_id) = type_.get_func_scope_id() {
                        let block = self.block(func_scope_id, ScopeKind::FuncBody)?;
                        DeclPostfix::Block(Box::new(block))
                    } else {
                        DeclPostfix::None
                    }
                },
                _ => DeclPostfix::None,
            };

            if !matches!(*self.traveler.head().kind(), TokenKind::Comma) {
                decls.push(Decl { type_, postfix });
                break;
            } else {
                let mut decl_type = type_.clone_base();
                std::mem::swap(&mut type_, &mut decl_type);

                decls.push(Decl { type_: decl_type, postfix });
                self.traveler.move_forward()?;
            }
        }
        Ok(decls)
    }

    // region: Type Parsing
    fn type_base(&mut self, scope_id: ScopeId, local: bool) -> MayUnwind<Type> {
        let default_storage = if local {
            StorageKind::Auto
        } else {
            StorageKind::Declared
        };
        let mut type_ = Type::new(default_storage);

        loop {
            let index = self.traveler.index();
            match *self.traveler.head().kind() {
                TokenKind::Keyword(keyword) => match keyword {
                    keyword if keyword.is_type_modifier() => type_.add_modifier(keyword, index),
                    Keyword::Alignas => {
                        if let TokenKind::LParen = *self.traveler.move_forward()?.kind() {
                            let alignment = self.expr(scope_id, true)?;
                            type_.segments.push(ModifierSegment::Alignas(alignment).into());
                        } else {
                            // TODO: Error
                            todo!("AlignAs expects (")
                        }
                    },
                    keyword if keyword.is_base_type() => {
                        type_.try_set_base_type(keyword, index);
                    },
                    keyword if keyword.is_type_tag() => {
                        let type_index = self.type_decl(scope_id)?;
                        type_.root = TypeRoot::Type(type_index);
                        type_.root_index = Some(index);
                        continue;
                    },
                    keyword if keyword.is_storage_class() => {
                        if !type_.storage.try_set(keyword, index) {
                            // TODO: Error
                            todo!("Multiple storage classes")
                        }
                    },
                    _ => break,
                },
                TokenKind::Identifier(ref id) => {
                    if let Some(decl_index) = self.file.find_decl_index(scope_id, id) {
                        let decl = self.file.get_decl(decl_index);
                        if !(decl.is_typedef() || type_.root_index.is_some()) {
                            break;
                        }

                        type_.root_index = Some(self.traveler.index());
                        type_.root = TypeRoot::Typedef(decl_index);
                        self.traveler.move_forward()?;
                        continue;
                    }
                    break;
                },
                _ => break,
            }

            self.traveler.move_forward()?;
        }
        type_.base_segments = type_.segments.len();
        Ok(type_)
    }

    fn type_name(&mut self, mut type_: Type, scope_id: ScopeId) -> MayUnwind<Type> {
        let mut insert_points = Vec::new();
        loop {
            match *self.traveler.head().kind() {
                TokenKind::Keyword(keyword) => match keyword {
                    _ if keyword.is_type_modifier() => {
                        type_.add_modifier(keyword, self.traveler.index())
                    },
                    _ => {
                        // TODO: Error
                        todo!()
                    },
                },
                TokenKind::LParen => {
                    let start_index = self.traveler.index();
                    match *self.traveler.move_forward()?.kind() {
                        TokenKind::Identifier(..) => {
                            insert_points.push(type_.segments.len());
                            continue;
                        },
                        TokenKind::RParen => {
                            let func = self.type_func(scope_id, start_index)?;
                            type_.segments.push(func.into());
                            break;
                        },
                        _ => {
                            if self.is_head_a_type(scope_id) {
                                let func = self.type_func(scope_id, start_index)?;
                                type_.segments.push(func.into());
                                break;
                            } else {
                                insert_points.push(type_.segments.len());
                                continue;
                            }
                        },
                    }
                },
                TokenKind::Star => {
                    let index = self.traveler.index();
                    type_.segments.push(PointerSegment(index).into());
                },
                TokenKind::Identifier(ref id) => {
                    type_.name = Some(id.clone());
                    self.traveler.move_forward()?;
                    break;
                },
                TokenKind::RParen
                | TokenKind::LBracket { .. }
                | TokenKind::Comma
                | TokenKind::Semicolon => break,
                _ => {
                    // TODO: Error
                    todo!("{:?}", self.traveler.head());
                },
            }

            self.traveler.move_forward()?;
        }
        insert_points.push(type_.segments.len());

        loop {
            match *self.traveler.head().kind() {
                TokenKind::LParen => {
                    let start_index = self.traveler.index();
                    self.traveler.move_forward()?;
                    let func = self.type_func(scope_id, start_index)?;
                    let insert_at = insert_points.last_mut().unwrap();
                    type_.segments.insert(*insert_at, func.into());
                    insert_at.increment();
                    break;
                },
                TokenKind::RParen if insert_points.len() >= 2 => {
                    insert_points.pop();
                },
                TokenKind::LBracket { .. } => {
                    let array = self.type_array(scope_id)?;
                    let insert_at = insert_points.last_mut().unwrap();
                    type_.segments.insert(*insert_at, array.into());
                    insert_at.increment();
                    break;
                },
                _ => break,
            }

            self.traveler.move_forward()?;
        }

        Ok(type_)
    }

    fn type_array(&mut self, scope_id: ScopeId) -> MayUnwind<ArraySegment> {
        let start_index = self.traveler.index();

        let mut const_ = None;
        let mut restrict = None;
        let mut static_ = None;
        while let TokenKind::Keyword(keyword) = *self.traveler.move_forward()?.kind() {
            match keyword {
                Keyword::Const if const_.is_none() => {
                    const_ = Some(self.traveler.index());
                },
                Keyword::Restrict if restrict.is_none() => {
                    restrict = Some(self.traveler.index());
                },
                Keyword::Static if static_.is_none() => {
                    static_ = Some(self.traveler.index());
                },
                Keyword::Const | Keyword::Restrict | Keyword::Static => {
                    // TODO: Duplicate modifier error
                    todo!()
                },
                _ => break,
            }
        }

        let kind = match *self.traveler.head().kind() {
            TokenKind::RBracket { .. } => ArrayKind::Empty,
            TokenKind::Star => {
                let index = self.traveler.index();
                self.traveler.move_forward()?;
                ArrayKind::Star(index)
            },
            _ => {
                let expr = self.expr(scope_id, true)?;
                ArrayKind::Expr(expr)
            },
        };

        if matches!(*self.traveler.head().kind(), TokenKind::RBracket { .. }) {
            self.traveler.move_forward()?;
        } else {
            // TODO: Report error
            todo!()
        }

        let range = start_index..self.traveler.index();
        Ok(ArraySegment {
            range,
            const_,
            restrict,
            static_,
            kind,
        })
    }

    fn type_func(
        &mut self,
        parent_id: ScopeId,
        start_index: TravelIndex,
    ) -> MayUnwind<FuncSegment> {
        // TODO: Parse K&R functions

        // A function type can appear in a function's arguments as a func-pointer.
        let parent_id = self.file.nearest_non_func_decl_scope(parent_id);
        // NOTE: This function should be called after the (
        let scope_id = self.file.new_scope(parent_id, ScopeKind::FuncDecl);
        let mut decls = Vec::new();
        let mut vararg_index = None;
        loop {
            match *self.traveler.head().kind() {
                TokenKind::RParen => {
                    if !decls.is_empty() {
                        // TODO: Error ,)
                        todo!()
                    }
                    break;
                },
                TokenKind::LBrace { .. } => break,
                TokenKind::DotDotDot => {
                    vararg_index = Some(self.traveler.index());
                    break;
                },
                _ => {},
            }

            let mut type_ = self.type_base(scope_id, true)?;
            type_ = self.type_name(type_, scope_id)?;
            decls.push(Decl { type_, postfix: DeclPostfix::None });

            match *self.traveler.head().kind() {
                TokenKind::RParen | TokenKind::LBrace { .. } => break,
                TokenKind::Comma => {
                    self.traveler.move_forward()?;
                },
                _ => {
                    // TODO: Error
                    todo!();
                },
            }
        }

        let scope = self.file.get_scope_mut(scope_id);
        scope.add_decls(decls);

        match *self.traveler.head().kind() {
            TokenKind::RParen => {
                self.traveler.move_forward()?;
            },
            TokenKind::LBrace { .. } => {
                // TODO: Error
                todo!()
            },
            _ => {
                // TODO: Error
                todo!();
            },
        }

        let range = start_index..self.traveler.index();

        Ok(FuncSegment { range, scope_id, vararg_index })
    }

    fn type_decl(&mut self, scope_id: ScopeId) -> MayUnwind<DeclIndex> {
        let type_kind = match *self.traveler.head().kind() {
            TokenKind::Keyword(Keyword::Enum) => TypeDeclKind::Enum,
            TokenKind::Keyword(Keyword::Struct) => TypeDeclKind::Struct,
            TokenKind::Keyword(Keyword::Union) => TypeDeclKind::Union,
            _ => {
                // TODO: Internal error
                todo!()
            },
        };
        let tag_index = self.traveler.index();
        let name = match *self.traveler.move_forward()?.kind() {
            TokenKind::Identifier(ref id) => {
                let result = Some(id.clone());
                self.traveler.move_forward()?;
                result
            },
            TokenKind::LBrace { .. } => None,
            _ => {
                // TODO: Error
                todo!()
            },
        };
        let tag = TypeDeclTag {
            range: tag_index..self.traveler.index(),
            kind: type_kind,
        };

        let type_decl = TypeDecl::new(tag, name);
        let has_body = matches!(*self.traveler.head().kind(), TokenKind::LBrace { .. });
        let index = self.file.add_type_decl(scope_id, type_decl, has_body);

        if has_body {
            use TypeDeclKind::*;
            let body = match type_kind {
                Struct | Union => self.type_decl_body(scope_id, type_kind)?,
                Enum => self.enum_decl_body(scope_id, index)?,
            };
            self.file.get_type_decl_mut(index).body = Some(body);
        }

        Ok(index)
    }

    fn type_decl_body(&mut self, scope_id: ScopeId, kind: TypeDeclKind) -> MayUnwind<TypeDeclBody> {
        let start_index = self.traveler.index();
        // Move past the {
        self.traveler.move_forward()?;

        let mut body = TypeDeclBody::new(kind);

        loop {
            match *self.traveler.head().kind() {
                TokenKind::RBrace { .. } | TokenKind::Eof => break,
                TokenKind::Semicolon => {
                    // TODO: Empty statement
                    self.traveler.move_forward()?;
                },
                _ => {
                    let decls = self.decls(scope_id, false)?;
                    body.add_decls(&self.file, decls);
                    match *self.traveler.head().kind() {
                        TokenKind::Semicolon => {
                            self.traveler.move_forward()?;
                        },
                        _ => {
                            // TODO: Error
                            todo!()
                        },
                    }
                },
            }
        }

        match *self.traveler.head().kind() {
            TokenKind::RBrace { .. } => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error
                todo!()
            },
        }
        body.range = start_index..self.traveler.index();
        Ok(body)
    }

    fn enum_decl_body(
        &mut self,
        scope_id: ScopeId,
        type_index: DeclIndex,
    ) -> MayUnwind<TypeDeclBody> {
        let start_index = self.traveler.index();
        // Move past the {
        self.traveler.move_forward()?;

        let mut body = TypeDeclBody::new(TypeDeclKind::Enum);

        loop {
            let id = match *self.traveler.head().kind() {
                TokenKind::Identifier(ref id) => id.clone(),
                TokenKind::Comma => {
                    // TODO: Error, two commas in a row
                    self.traveler.move_forward()?;
                    continue;
                },
                TokenKind::RBrace { .. } => break,
                _ => {
                    // TODO: Error
                    todo!()
                },
            };

            let postfix = if matches!(*self.traveler.move_forward()?.kind(), TokenKind::Equal) {
                self.traveler.move_forward()?;
                DeclPostfix::Initializer(self.expr(scope_id, false)?)
            } else {
                DeclPostfix::None
            };

            body.fields.add_keyed(
                id.clone(),
                Decl {
                    type_: Type::new_enum(id.clone()),
                    postfix,
                }
                .into(),
            );

            self.file
                .get_scope_mut(scope_id)
                .decls
                .add_keyed(id.clone(), Decl::new_enum_forward(id, type_index));

            match *self.traveler.head().kind() {
                TokenKind::RBrace { .. } | TokenKind::Eof => break,
                TokenKind::Comma => {
                    self.traveler.move_forward()?;
                },
                _ => {
                    // TODO: Error
                    todo!()
                },
            }
        }

        match *self.traveler.head().kind() {
            TokenKind::RBrace { .. } => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error
                todo!()
            },
        }
        body.range = start_index..self.traveler.index();
        Ok(body)
    }
    // endregion: Type Parsing

    // region: Statement Parsing
    fn stmt(&mut self, scope_id: ScopeId) -> MayUnwind<Stmt> {
        let stmt: Stmt = match *self.traveler.head().kind() {
            TokenKind::Keyword(keyword) => match keyword {
                Keyword::Break => self.break_stmt(scope_id)?.into(),
                Keyword::Continue => self.continue_stmt(scope_id)?.into(),
                Keyword::Case => self.case_stmt(true, scope_id)?.into(),
                Keyword::Default => self.case_stmt(false, scope_id)?.into(),
                Keyword::Return => self.return_stmt(scope_id)?.into(),
                Keyword::Goto => self.goto_stmt(scope_id)?.into(),
                Keyword::If => self.if_stmt(scope_id)?.into(),
                Keyword::While => self.while_stmt(scope_id)?.into(),
                Keyword::For => self.for_stmt(scope_id)?.into(),
                Keyword::Do => self.do_stmt(scope_id)?.into(),
                Keyword::Switch => self.switch_stmt(scope_id)?.into(),
                Keyword::StaticAssert => {
                    todo!("_Static_assert")
                },
                _ if keyword.is_type_starter() => self.decl_stmt(scope_id)?.into(),
                _ => (*self.expr(scope_id, true)?).into(),
            },
            TokenKind::LBrace { .. } => self.block(scope_id, ScopeKind::Block)?.into(),
            TokenKind::Identifier(ref id) => match self.file.find_decl(scope_id, id) {
                Some(decl) if decl.is_typedef() => self.decl_stmt(scope_id)?.into(),
                _ => (*self.expr(scope_id, true)?).into(),
            },
            TokenKind::Semicolon => {
                let index = self.traveler.index();
                self.traveler.move_forward()?;
                index.into()
            },
            _ => (*self.expr(scope_id, true)?).into(),
        };

        match *self.traveler.head().kind() {
            _ if !stmt.requires_semicolon() => {},
            TokenKind::Semicolon => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error, expected ; to separate statements
                todo!("{:?}", self.traveler.head())
            },
        }

        Ok(stmt)
    }

    fn break_stmt(&mut self, scope_id: ScopeId) -> MayUnwind<BreakStmt> {
        let break_index = self.traveler.index();
        self.traveler.move_forward()?;
        let break_scope_id = self.file.find_scope_kind(scope_id, |kind| kind.is_breakable());
        Ok(BreakStmt { break_scope_id, break_index })
    }

    fn continue_stmt(&mut self, scope_id: ScopeId) -> MayUnwind<ContinueStmt> {
        let continue_index = self.traveler.index();
        self.traveler.move_forward()?;
        let continue_scope_id = self.file.find_scope_kind(scope_id, |kind| kind.is_continuable());
        Ok(ContinueStmt { continue_scope_id, continue_index })
    }

    fn case_stmt(&mut self, has_expr: bool, scope_id: ScopeId) -> MayUnwind<CaseStmt> {
        let start_index = self.traveler.index();
        let switch_scope = self.file.find_scope_kind(scope_id, |kind| kind == ScopeKind::Switch);
        self.traveler.move_forward()?;
        let expr = if has_expr {
            Some(self.expr(scope_id, true)?)
        } else {
            None
        };

        match *self.traveler.head().kind() {
            TokenKind::Colon => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error
                todo!()
            },
        }

        let stmt = if matches!(*self.traveler.head().kind(), TokenKind::RBrace { .. }) {
            // TODO: Error label at end of block
            self.traveler.index().into()
        } else {
            self.stmt(scope_id)?
        };

        Ok(CaseStmt {
            range: start_index..self.traveler.index(),
            case: expr,
            stmt: Box::new(stmt),
            switch_scope,
        })
    }

    fn return_stmt(&mut self, scope_id: ScopeId) -> MayUnwind<ReturnStmt> {
        let return_index = self.traveler.index();
        let expr = match *self.traveler.move_forward()?.kind() {
            TokenKind::RBrace { .. } | TokenKind::Semicolon => None,
            _ => Some(self.expr(scope_id, true)?),
        };
        Ok(ReturnStmt { return_index, expr })
    }

    fn goto_stmt(&mut self, scope_id: ScopeId) -> MayUnwind<GotoStmt> {
        let start_index = self.traveler.index();
        let label = match *self.traveler.move_forward()?.kind() {
            TokenKind::Identifier(ref id) => {
                let id = id.clone();
                self.traveler.move_forward()?;
                Some(id)
            },
            TokenKind::Semicolon => {
                // TODO: Missing id
                None
            },
            _ => {
                // TODO: Expected id to follow.
                return Err(Unwind::Block);
            },
        };
        let label_scope_id = self.file.find_scope_kind(scope_id, |kind| kind.manages_labels());
        let range = start_index..self.traveler.index();
        Ok(GotoStmt { range, label_scope_id, label })
    }

    fn if_stmt(&mut self, parent_id: ScopeId) -> MayUnwind<IfStmt> {
        let scope_id = self.file.new_scope(parent_id, ScopeKind::Block);

        let start_index = self.traveler.index();
        self.traveler.move_forward()?;
        let condition = self.condition(scope_id)?;
        let block = Box::new(self.stmt(scope_id)?);
        let else_ = if matches!(
            *self.traveler.head().kind(),
            TokenKind::Keyword(Keyword::Else)
        ) {
            let else_scope_id = self.file.new_scope(parent_id, ScopeKind::Block);
            self.traveler.move_forward()?;
            Some(Box::new(self.stmt(else_scope_id)?))
        } else {
            None
        };
        let range = start_index..self.traveler.index();
        Ok(IfStmt { range, condition, block, else_ })
    }

    fn while_stmt(&mut self, parent_id: ScopeId) -> MayUnwind<WhileStmt> {
        let scope_id = self.file.new_scope(parent_id, ScopeKind::Loop);

        let start_index = self.traveler.index();
        self.traveler.move_forward()?;
        let condition = self.condition(scope_id)?;
        let block = Box::new(self.stmt(scope_id)?);
        let range = start_index..self.traveler.index();
        Ok(WhileStmt { range, condition, block })
    }

    fn do_stmt(&mut self, parent_id: ScopeId) -> MayUnwind<DoStmt> {
        let scope_id = self.file.new_scope(parent_id, ScopeKind::Loop);

        let start_index = self.traveler.index();
        self.traveler.move_forward()?;
        let block = Box::new(self.stmt(scope_id)?);

        let condition = if matches!(
            *self.traveler.head().kind(),
            TokenKind::Keyword(Keyword::While)
        ) {
            self.traveler.move_forward()?;
            self.condition(scope_id)?
        } else {
            // TODO: Report missing while condition
            Box::new(self.number_expr("0", None)?.into())
        };

        let range = start_index..self.traveler.index();
        Ok(DoStmt { range, block, condition })
    }

    fn for_stmt(&mut self, parent_id: ScopeId) -> MayUnwind<ForStmt> {
        let start_index = self.traveler.index();
        let scope_id = self.file.new_scope(parent_id, ScopeKind::Loop);

        match *self.traveler.move_forward()?.kind() {
            TokenKind::LParen => {
                self.traveler.move_forward()?;
            },
            TokenKind::LBrace { .. } => {
                // TODO: Report error
                todo!()
            },
            _ => {
                // TODO: Report error
                todo!()
            },
        }

        let initializer = self.stmt(scope_id)?;

        let condition = match *self.traveler.head().kind() {
            TokenKind::Semicolon => {
                self.traveler.move_forward()?;
                None
            },
            _ => {
                let condition = self.expr(scope_id, true)?;
                if matches!(*self.traveler.head().kind(), TokenKind::Semicolon) {
                    self.traveler.move_forward()?;
                } else {
                    // TODO: Error
                    todo!()
                }
                Some(condition)
            },
        };

        let increment = match *self.traveler.head().kind() {
            TokenKind::RParen | TokenKind::LBrace { .. } => None,
            _ => Some(self.expr(scope_id, true)?),
        };

        match *self.traveler.head().kind() {
            TokenKind::RParen => {
                self.traveler.move_forward()?;
            },
            TokenKind::LBrace { .. } => {
                // TODO: Report missing )
            },
            _ => {
                // TODO: Report error
                todo!()
            },
        }

        let block = self.stmt(scope_id)?;
        let range = start_index..self.traveler.index();

        Ok(ForStmt {
            range,
            initial: Box::new(initializer),
            condition,
            increment,
            block: Box::new(block),
        })
    }

    fn switch_stmt(&mut self, parent_id: ScopeId) -> MayUnwind<SwitchStmt> {
        let scope_id = self.file.new_scope(parent_id, ScopeKind::Switch);

        let start_index = self.traveler.index();
        self.traveler.move_forward()?;
        let value = self.condition(scope_id)?;
        let block = Box::new(self.stmt(scope_id)?);

        let range = start_index..self.traveler.index();
        Ok(SwitchStmt { range, value, block })
    }

    fn condition(&mut self, scope_id: ScopeId) -> MayUnwind<Box<Expr>> {
        match *self.traveler.head().kind() {
            TokenKind::LBrace { .. } => {
                // TODO: Report missing condition
                Ok(Box::new(self.number_expr("0", None)?.into()))
            },
            TokenKind::LParen => {
                // parse_parens requires the caller to move beyond the left parenthesis.
                let start_index = self.traveler.index();
                self.traveler.move_forward()?;
                Ok(Box::new(self.parens_expr(start_index, scope_id)?.into()))
            },
            _ => {
                // TODO: Report missing (
                let expr = self.expr(scope_id, true)?;
                match *self.traveler.head().kind() {
                    TokenKind::RParen => {
                        self.traveler.move_forward()?;
                    },
                    TokenKind::LBrace { .. } => {},
                    _ => {
                        // TODO: Report missing )
                    },
                }
                Ok(expr)
            },
        }
    }

    fn decl_stmt(&mut self, scope_id: ScopeId) -> MayUnwind<DeclStmt> {
        let start_index = self.traveler.index();

        let decls = self.decls(scope_id, true)?;
        let requires_semicolon = !decls.last().unwrap().is_function();
        let scope = self.file.get_scope_mut(scope_id);
        let decl_ids = scope.add_decls(decls);

        if requires_semicolon {
            match *self.traveler.head().kind() {
                TokenKind::Semicolon => {
                    self.traveler.move_forward()?;
                },
                _ => {
                    // TODO: Expected ; to end declaration
                    todo!("{:?}", self.traveler.head())
                },
            }
        }

        Ok(DeclStmt {
            range: start_index..self.traveler.index(),
            scope_id,
            decl_ids,
        })
    }

    fn block(&mut self, parent_id: ScopeId, kind: ScopeKind) -> MayUnwind<BlockExpr> {
        let scope_id = self.file.new_scope(parent_id, kind);
        let start_index = self.traveler.index();
        // Move past the {
        self.traveler.move_forward()?;

        loop {
            match *self.traveler.head().kind() {
                TokenKind::Semicolon => {
                    self.traveler.move_forward()?;
                    // TODO: Warn on empty statement
                },
                TokenKind::RBrace { .. } | TokenKind::Eof => break,
                _ => {
                    let stmt = self.stmt(scope_id)?;
                    self.file.get_scope_mut(scope_id).stmts.push(stmt);
                },
            }
        }

        if matches!(*self.traveler.head().kind(), TokenKind::RBrace { .. }) {
            self.traveler.move_forward()?;
        } else {
            // TODO: Report error
            todo!()
        }

        let range = start_index..self.traveler.index();
        Ok(BlockExpr { range, scope_id })
    }
    // endregion: Statement Parsing

    // region: Expression Parsing
    fn expr(&mut self, scope_id: ScopeId, comma_support: bool) -> MayUnwind<Box<Expr>> {
        let mut expr = self.expr_atom(scope_id)?;
        loop {
            let head = self.traveler.head().kind();
            expr = if let Ok(op) = head.try_into::<BinaryOp>() {
                if op == BinaryOp::Comma && !comma_support {
                    break;
                }
                let op_index = self.traveler.index();
                self.traveler.move_forward()?;
                let rhs = self.expr_atom(scope_id)?;
                expr.add_op(op.precedence(), |lhs| {
                    Box::new(BinaryExpr { lhs, op, op_index, rhs }.into())
                })
            } else if let Ok(op) = head.try_into() {
                let op_index = self.traveler.index();
                self.traveler.move_forward()?;
                let value = self.expr_atom(scope_id)?;
                expr.add_op(Precedence::Assignment, |to| {
                    Box::new(AssignExpr { to, op, op_index, value }.into())
                })
            } else if let Ok(op) = head.try_into() {
                let op_index = self.traveler.index();
                self.traveler.move_forward()?;
                expr.add_op(Precedence::Suffixes, |expr| {
                    Box::new(SuffixExpr { expr, op, op_index }.into())
                })
            } else {
                use TokenKind::*;
                match *head {
                    Dot => self.access_expr(false, expr)?,
                    Arrow => self.access_expr(true, expr)?,
                    LBracket { .. } => self.array_expr(scope_id, expr)?,
                    LParen => self.call_expr(scope_id, expr)?,
                    QMark => self.ternary_expr(scope_id, expr)?,
                    _ => break,
                }
            }
        }
        Ok(expr)
    }

    fn access_expr(&mut self, through_ptr: bool, expr: Box<Expr>) -> MayUnwind<Box<Expr>> {
        let start_index = self.traveler.index();
        let member = match *self.traveler.move_forward()?.kind() {
            TokenKind::Identifier(ref id) => id.clone(),
            _ => {
                // TODO: Report error
                return Err(Unwind::Block);
            },
        };
        self.traveler.move_forward()?;

        Ok(expr.add_op(Precedence::Suffixes, |expr| {
            let new_expr = AccessExpr {
                member,
                through_ptr,
                expr,
                range: start_index..self.traveler.index(),
            };
            Box::new(new_expr.into())
        }))
    }

    fn array_expr(&mut self, scope_id: ScopeId, expr: Box<Expr>) -> MayUnwind<Box<Expr>> {
        let start_index = self.traveler.index();
        let offset = match *self.traveler.move_forward()?.kind() {
            TokenKind::RBracket { .. } => {
                // TODO: Error about missing expression.
                todo!()
            },
            _ => self.expr(scope_id, true)?,
        };

        match *self.traveler.head().kind() {
            TokenKind::RBracket { .. } => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error about unended array
                todo!()
            },
        }

        Ok(expr.add_op(Precedence::Suffixes, |expr| {
            let new_expr = ArrayExpr {
                range: start_index..self.traveler.index(),
                expr,
                offset,
            };
            Box::new(new_expr.into())
        }))
    }

    fn call_expr(&mut self, scope_id: ScopeId, expr: Box<Expr>) -> MayUnwind<Box<Expr>> {
        let start_index = self.traveler.index();
        self.traveler.move_forward()?;
        let mut args = Vec::new();
        loop {
            match *self.traveler.head().kind() {
                TokenKind::Comma => {
                    // TODO: Error about missing parameter
                    todo!()
                },
                TokenKind::RParen | TokenKind::Eof => {
                    // TODO: Error expected parameter expression
                    break;
                },
                _ => {
                    args.push(*self.expr(scope_id, false)?);
                },
            }

            match *self.traveler.head().kind() {
                TokenKind::Comma => {
                    self.traveler.move_forward()?;
                },
                TokenKind::RParen | TokenKind::Eof => break,
                _ => {
                    // TODO: Error
                    todo!()
                },
            }
        }

        match *self.traveler.head().kind() {
            TokenKind::RParen => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error
                todo!()
            },
        }

        Ok(expr.add_op(Precedence::Suffixes, |expr| {
            let new_expr = CallExpr {
                range: start_index..self.traveler.index(),
                expr,
                args,
            };
            Box::new(new_expr.into())
        }))
    }

    fn ternary_expr(&mut self, scope_id: ScopeId, expr: Box<Expr>) -> MayUnwind<Box<Expr>> {
        let qmark_index = self.traveler.index();
        // Move past the ?
        self.traveler.move_forward()?;
        let if_true = self.expr(scope_id, true)?;

        let maybe_colon = self.traveler.head().kind();
        let colon_index = if matches!(*maybe_colon, TokenKind::Colon) {
            self.traveler.index()
        } else {
            // TODO: Ternary expects colon error
            return Err(Unwind::Block);
        };
        // Move past the :
        self.traveler.move_forward()?;

        let if_false = self.expr_atom(scope_id)?;

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

    fn expr_atom(&mut self, scope_id: ScopeId) -> MayUnwind<Box<Expr>> {
        if let Ok(op) = self.traveler.head().kind().try_into() {
            return Ok(Box::new(self.prefix_expr(scope_id, op)?.into()));
        } else if let Ok(op) = self.traveler.head().kind().try_into() {
            return Ok(Box::new(self.type_expr(scope_id, op)?.into()));
        }

        match *self.traveler.head().kind() {
            TokenKind::Number(ref digits) => {
                let digits = digits.clone();
                Ok(Box::new(self.number_expr(digits.string(), None)?.into()))
            },
            TokenKind::String {
                is_char: true,
                ref str_data,
                encoding,
                ..
            } => {
                let digits = str_data.clone();
                Ok(Box::new(
                    self.number_expr(&*digits, Some(encoding))?.into(),
                ))
            },
            TokenKind::String { .. } => Ok(Box::new(self.string_expr()?.into())),
            TokenKind::LParen { .. } => {
                let start_index = self.traveler.index();
                self.traveler.move_forward()?;
                match *self.traveler.head().kind() {
                    _ if self.is_head_a_type(scope_id) => {
                        Ok(Box::new(self.cast_expr(start_index, scope_id)?.into()))
                    },
                    TokenKind::LBrace { .. } => {
                        Ok(Box::new(self.block_expr(start_index, scope_id)?.into()))
                    },
                    _ => Ok(Box::new(self.parens_expr(start_index, scope_id)?.into())),
                }
            },
            TokenKind::LBrace { .. } => Ok(Box::new(self.init_expr(scope_id)?.into())),
            TokenKind::Keyword(Keyword::Generic) => {
                // TODO: Parse generic
                todo!("_Generic")
            },
            TokenKind::Identifier(ref id) => {
                let id = Id {
                    text: id.clone(),
                    index: self.traveler.index(),
                };
                let decl_id = self.file.find_decl_index(scope_id, &id.text);
                self.traveler.move_forward()?;

                Ok(Box::new(DeclRefExpr { id, decl_id }.into()))
            },
            _ => {
                // TODO: Error
                todo!("Atom error {:?}", self.traveler.head())
            },
        }
    }

    fn number_expr(&mut self, digits: &str, enc: Option<StringEnc>) -> MayUnwind<Number> {
        let index = self.traveler.index();
        let mut error_callback = |err: NumberError| self.report_error(err.into()).is_err();
        let kind = if let Some(enc) = enc {
            NumberKind::from_character(&*digits, enc, &mut error_callback)?
        } else {
            NumberKind::from_number(&*digits, &mut error_callback)?
        };
        self.traveler.move_forward()?;
        Ok(Number { kind, index })
    }

    fn string_expr(&mut self) -> MayUnwind<StringLiteral> {
        let start_index = self.traveler.index();
        let mut joined_encoding = StringEnc::Default;
        let mut has_any_escapes = false;
        let mut strings = SmallVec::new();
        while let TokenKind::String {
            is_char: false,
            encoding,
            has_escapes,
            ref str_data,
        } = *self.traveler.head().kind()
        {
            has_any_escapes |= has_escapes;
            strings.push(str_data.clone());
            match encoding {
                StringEnc::Default => {},
                _ if encoding == joined_encoding => {},
                _ if joined_encoding != StringEnc::Default => {
                    // TODO: Report missed encodings
                    todo!()
                },
                _ => {
                    joined_encoding = encoding;
                },
            }
            self.traveler.move_forward()?;
        }

        let range = start_index..self.traveler.index();

        Ok(StringLiteral {
            range,
            encoding: joined_encoding,
            segments: strings,
            has_escapes: has_any_escapes,
        })
    }

    fn parens_expr(&mut self, start_index: TravelIndex, scope_id: ScopeId) -> MayUnwind<ParenExpr> {
        // This function should have been called after the (.
        let expr = self.expr(scope_id, true)?;
        if matches!(*self.traveler.head().kind(), TokenKind::RParen) {
            self.traveler.move_forward()?;
        } else {
            // TODO: Report error
            todo!()
        }
        let range = start_index..self.traveler.index();
        Ok(ParenExpr { range, expr })
    }

    fn init_expr(&mut self, scope_id: ScopeId) -> MayUnwind<InitExpr> {
        let start_index = self.traveler.index();
        self.traveler.move_forward()?;

        let mut members = Vec::new();
        loop {
            let member = match *self.traveler.head().kind() {
                TokenKind::RBrace { .. } | TokenKind::Eof => break,
                TokenKind::Dot => {
                    self.traveler.move_forward()?;
                    let text = match *self.traveler.head().kind() {
                        TokenKind::Identifier(ref id) => id.clone(),
                        _ => {
                            // TODO: Error
                            todo!()
                        },
                    };
                    let index = self.traveler.index();
                    let id = Id { text, index };

                    match *self.traveler.move_forward()?.kind() {
                        TokenKind::Equal => {
                            self.traveler.move_forward()?;
                        },
                        TokenKind::Comma => {
                            // TODO: Error
                            self.traveler.move_forward()?;
                            continue;
                        },
                        TokenKind::RBrace { .. } | TokenKind::Eof => break,
                        _ => {
                            // TODO: Error missing =
                        },
                    }

                    let expr = self.expr(scope_id, false)?;
                    InitMember::Named(id, *expr)
                },
                TokenKind::LBracket { .. } => {
                    let mut indexes = SmallVec::new();
                    loop {
                        match *self.traveler.head().kind() {
                            TokenKind::LBracket { .. } => {
                                self.traveler.move_forward()?;
                            },
                            TokenKind::Equal => {
                                self.traveler.move_forward()?;
                                break;
                            },
                            _ => {
                                // TODO: Error
                                todo!()
                            },
                        }

                        let expr = self.expr(scope_id, true)?;
                        indexes.push(*expr);
                        match *self.traveler.move_forward()?.kind() {
                            TokenKind::RBracket { .. } => {
                                self.traveler.move_forward()?;
                            },
                            _ => {
                                // TODO: Error
                                todo!()
                            },
                        }
                    }

                    let expr = self.expr(scope_id, false)?;
                    InitMember::Array(indexes, *expr)
                },
                TokenKind::LBrace { .. } => {
                    let subinit = self.init_expr(scope_id)?;
                    InitMember::SubInitializer(subinit)
                },
                _ => {
                    let expr = self.expr(scope_id, false)?;
                    InitMember::Unnamed(*expr)
                },
            };
            members.push(member);

            match *self.traveler.head().kind() {
                TokenKind::Comma => {
                    self.traveler.move_forward()?;
                },
                TokenKind::RBrace { .. } | TokenKind::Eof => break,
                _ => {
                    // TODO: Error
                    todo!()
                },
            }
        }

        match *self.traveler.head().kind() {
            TokenKind::RBrace { .. } => {
                self.traveler.move_forward()?;
            },
            _ => {
                // TODO: Error
                todo!()
            },
        }

        Ok(InitExpr {
            range: start_index..self.traveler.index(),
            values: members,
        })
    }

    fn block_expr(&mut self, start_index: TravelIndex, scope_id: ScopeId) -> MayUnwind<BlockExpr> {
        // This function should have been called after the (.
        let mut block = self.block(scope_id, ScopeKind::Block)?;
        block.range.start = start_index;
        if matches!(*self.traveler.head().kind(), TokenKind::RParen) {
            self.traveler.move_forward()?;
            block.range.end = self.traveler.index();
        } else {
            // TODO: Report error
            todo!()
        }
        Ok(block)
    }

    fn prefix_expr(&mut self, scope_id: ScopeId, op: PrefixOp) -> MayUnwind<PrefixExpr> {
        let op_index = self.traveler.index();
        self.traveler.move_forward()?;
        let expr = self.expr_atom(scope_id)?;
        let range = op_index..self.traveler.index();
        Ok(PrefixExpr { range, op, expr })
    }

    fn type_expr(&mut self, scope_id: ScopeId, op: TypeOp) -> MayUnwind<TypeExpr> {
        let op_index = self.traveler.index();
        let of = match *self.traveler.move_forward()?.kind() {
            TokenKind::LParen => {
                let paren_index = self.traveler.index();
                self.traveler.move_forward()?;
                if self.is_head_a_type(scope_id) {
                    let type_ = self.type_base(scope_id, true)?;
                    let type_ = self.type_name(type_, scope_id)?;

                    match *self.traveler.head().kind() {
                        TokenKind::RParen => {
                            self.traveler.move_forward()?;
                        },
                        _ => {
                            // TODO: Error
                            todo!()
                        },
                    }

                    type_.into()
                } else {
                    let expr: Box<Expr> = Box::new(self.parens_expr(paren_index, scope_id)?.into());
                    expr.into()
                }
            },
            _ => self.expr_atom(scope_id)?.into(),
        };
        Ok(TypeExpr {
            range: op_index..self.traveler.index(),
            op,
            of,
        })
    }

    fn cast_expr(&mut self, start_index: TravelIndex, scope_id: ScopeId) -> MayUnwind<CastExpr> {
        // This function should have been called after the (.
        let mut to = self.type_base(scope_id, true)?;
        to = self.type_name(to, scope_id)?;
        if matches!(*self.traveler.head().kind(), TokenKind::RParen) {
            self.traveler.move_forward()?;
        } else {
            // TODO: Report error
            todo!()
        }
        let expr = self.expr_atom(scope_id)?;
        let range = start_index..self.traveler.index();
        Ok(CastExpr { range, to, expr })
    }
    // endregion: Expression parsing

    fn is_head_a_type(&self, scope_id: ScopeId) -> bool {
        match *self.traveler.head().kind() {
            TokenKind::Keyword(keyword) => match keyword {
                _ if keyword.is_base_type() => true,
                _ if keyword.is_type_modifier() => true,
                _ if keyword.is_type_tag() => true,
                _ => false,
            },
            TokenKind::Identifier(ref id) => {
                matches!(
                    self.file.find_decl(scope_id, id),
                    Some(decl) if decl.is_typedef()
                )
            },
            _ => false,
        }
    }

    fn report_error(&mut self, error: Error) -> MayUnwind<()> {
        let full_error = ParseError {
            kind: error,
            state: self.traveler.save_state(),
        };
        self.errors.report(full_error)
    }
}

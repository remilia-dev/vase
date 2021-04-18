// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.

use smallvec::SmallVec;

use crate::{
    c::{
        ast::{
            BlockExpr,
            Expr,
            Id,
            ScopeId,
            StorageKind,
            Type,
            TypeRoot,
            TypeSegment,
        },
        TravelRange,
    },
    math::NonMaxU32,
    util::{
        CachedString,
        RedeclMapIndex,
    },
};

#[derive(Clone, Debug)]
pub struct Decl {
    pub type_: Type,
    pub postfix: DeclPostfix,
}

impl Decl {
    pub fn new_enum_forward(name: CachedString, type_index: DeclIndex) -> Self {
        let mut type_ = Type::new(StorageKind::Declared);
        type_.name = Some(name);
        type_.root = TypeRoot::EnumForward(type_index);
        Decl { type_, postfix: DeclPostfix::None }
    }

    pub fn is_typedef(&self) -> bool {
        matches!(self.type_.storage.kind, StorageKind::Typedef)
    }

    pub fn is_function(&self) -> bool {
        !self.is_typedef() && matches!(self.type_.segments.last(), Some(&TypeSegment::Func(..)))
    }
}

#[derive(Clone, Debug)]
pub enum DeclPostfix {
    None,
    Bitfield(Box<Expr>),
    Initializer(Box<Expr>),
    Block(Box<BlockExpr>),
}

#[derive(Clone, Debug)]
pub struct DeclStmt {
    pub range: TravelRange,
    pub scope_id: ScopeId,
    pub decl_ids: SmallVec<[RedeclMapIndex; 1]>,
}

#[derive(Clone, Debug)]
pub struct DeclRefExpr {
    pub id: Id,
    pub decl_id: Option<DeclIndex>,
}

#[derive(Copy, Clone, Debug)]
pub struct DeclIndex {
    pub scope_id: ScopeId,
    pub decl_index: NonMaxU32,
    pub redecl_index: NonMaxU32,
}

impl DeclIndex {
    pub fn new(scope_id: ScopeId, decl: RedeclMapIndex) -> Self {
        Self {
            scope_id,
            decl_index: decl.index,
            redecl_index: decl.redecl_index,
        }
    }
}

impl From<DeclIndex> for RedeclMapIndex {
    fn from(v: DeclIndex) -> Self {
        RedeclMapIndex {
            index: v.decl_index,
            redecl_index: v.redecl_index,
        }
    }
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use smallvec::SmallVec;

use crate::{
    c::ast::{
        Decl,
        Stmt,
        TypeDecl,
    },
    math::NonMaxU32,
    util::{
        CachedString,
        RedeclMap,
        RedeclMapIndex,
    },
};

pub type ScopeId = NonMaxU32;

#[derive(Clone, Debug)]
pub struct Scope {
    pub parent: Option<ScopeId>,
    pub kind: ScopeKind,
    pub stmts: Vec<Stmt>,
    pub types: RedeclMap<CachedString, TypeDecl>,
    pub decls: RedeclMap<CachedString, Decl>,
}

impl Scope {
    pub fn new_root() -> Self {
        Scope {
            parent: None,
            kind: ScopeKind::Global,
            stmts: Vec::new(),
            types: RedeclMap::default(),
            decls: RedeclMap::default(),
        }
    }

    pub fn new(parent: ScopeId, kind: ScopeKind) -> Self {
        Scope {
            parent: Some(parent),
            kind,
            stmts: Vec::new(),
            types: RedeclMap::default(),
            decls: RedeclMap::default(),
        }
    }

    pub fn kind(&self) -> ScopeKind {
        self.kind
    }

    pub fn add_decls<I>(&mut self, decls: I) -> SmallVec<[RedeclMapIndex; 1]>
    where I: IntoIterator<Item = Decl> {
        let mut res = SmallVec::new();

        for decl in decls {
            let index = self.decls.add(decl.type_.name.clone(), decl);
            res.push(index);
        }
        res
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ScopeKind {
    Global,
    FuncDecl,
    FuncBody,
    Block,
    Loop,
    Switch,
}

impl ScopeKind {
    pub fn is_breakable(self) -> bool {
        matches!(self, Self::Loop | Self::Switch)
    }

    pub fn is_continuable(self) -> bool {
        matches!(self, Self::Loop)
    }

    pub fn manages_labels(self) -> bool {
        matches!(self, Self::FuncBody)
    }

    pub fn is_switch(self) -> bool {
        matches!(self, Self::Switch)
    }
}

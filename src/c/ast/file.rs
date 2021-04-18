// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use crate::{
    c::ast::{
        Decl,
        DeclIndex,
        Scope,
        ScopeId,
        ScopeKind,
        TypeDecl,
    },
    math::NonMaxU32,
    sync::Arc,
    util::{
        CachedString,
        Conversions,
        FileId,
        Vec32,
    },
};

#[derive(Clone, Debug)]
pub struct SourceFile {
    file_id: FileId,
    scopes: Vec32<Scope>,
    path: Option<Arc<Path>>,
}

impl SourceFile {
    pub fn new(file_id: FileId, path: Option<Arc<Path>>) -> Self {
        let mut this = SourceFile { file_id, scopes: Vec32::new(), path };
        this.scopes.push(Scope::new_root());
        this
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn path(&self) -> &Option<Arc<Path>> {
        &self.path
    }

    pub fn root_scope(&self) -> &Scope {
        &self.scopes[0.into_type::<NonMaxU32>()]
    }

    pub fn root_scope_mut(&mut self) -> &mut Scope {
        &mut self.scopes[0.into_type::<NonMaxU32>()]
    }

    pub fn get_scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id]
    }

    pub fn get_scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id]
    }

    pub fn new_scope(&mut self, parent_id: ScopeId, kind: ScopeKind) -> ScopeId {
        let id = self.scopes.len();
        self.scopes.push(Scope::new(parent_id, kind));
        id
    }

    pub fn search_scopes<W, T>(&self, scope_id: ScopeId, mut when: W) -> Option<T>
    where W: FnMut(&Scope, ScopeId) -> Option<T> {
        let scope = self.get_scope(scope_id);
        if let Some(value) = when(scope, scope_id) {
            Some(value)
        } else if let Some(parent_id) = scope.parent {
            self.search_scopes(parent_id, when)
        } else {
            None
        }
    }

    pub fn find_scope_kind<W>(&self, scope_id: ScopeId, mut when: W) -> Option<ScopeId>
    where W: FnMut(ScopeKind) -> bool {
        self.search_scopes(
            scope_id,
            |scope, id| {
                if when(scope.kind) { Some(id) } else { None }
            },
        )
    }

    pub fn nearest_non_func_decl_scope(&self, scope_id: ScopeId) -> ScopeId {
        self.find_scope_kind(scope_id, |kind| kind != ScopeKind::FuncDecl)
            .expect("All scopes should have led to the root scope (which is not a function scope).")
    }

    pub fn find_decl(&self, scope_id: ScopeId, id: &CachedString) -> Option<&Decl> {
        let index = self.find_decl_index(scope_id, id)?;
        Some(self.get_decl(index))
    }

    pub fn find_decl_index(&self, scope_id: ScopeId, id: &CachedString) -> Option<DeclIndex> {
        self.search_scopes(scope_id, |scope, scope_id| {
            let decl = scope.decls.get_index(id)?;
            Some(DeclIndex::new(scope_id, decl))
        })
    }

    pub fn find_type_decl_index(&self, scope_id: ScopeId, id: &CachedString) -> Option<DeclIndex> {
        self.search_scopes(scope_id, |scope, scope_id| {
            let decl = scope.types.get_index(id)?;
            Some(DeclIndex::new(scope_id, decl))
        })
    }

    pub fn get_decl(&self, index: DeclIndex) -> &Decl {
        &self.get_scope(index.scope_id).decls[index.into()]
    }

    pub fn get_decl_mut(&mut self, index: DeclIndex) -> &mut Decl {
        &mut self.get_scope_mut(index.scope_id).decls[index.into()]
    }

    pub fn get_type_decl(&self, index: DeclIndex) -> &TypeDecl {
        &self.get_scope(index.scope_id).types[index.into()]
    }

    pub fn get_type_decl_mut(&mut self, index: DeclIndex) -> &mut TypeDecl {
        &mut self.get_scope_mut(index.scope_id).types[index.into()]
    }

    pub fn add_type_decl(
        &mut self,
        scope_id: ScopeId,
        mut decl: TypeDecl,
        has_body: bool,
    ) -> DeclIndex {
        let name = match decl.name {
            Some(ref name) => name.clone(),
            None => return self.add_new_type_decl(scope_id, decl),
        };

        let index = match self.find_type_decl_index(scope_id, &name) {
            Some(index) => index,
            _ => return self.add_new_type_decl(scope_id, decl),
        };

        let type_decl = self.get_type_decl_mut(index);
        if has_body && !type_decl.incomplete() {
            return self.add_new_type_decl(scope_id, decl);
        } else {
            type_decl.tags.append(&mut decl.tags);
        }
        index
    }

    fn add_new_type_decl(&mut self, scope_id: ScopeId, decl: TypeDecl) -> DeclIndex {
        let name = decl.name.clone();
        let index = self.get_scope_mut(scope_id).types.add(name, decl);
        DeclIndex::new(scope_id, index)
    }
}

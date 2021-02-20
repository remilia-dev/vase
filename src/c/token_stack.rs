// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::HashMap;
use std::path::Path;

use crate::{
    c::{
        CLexerError,
        CToken,
        CTokenKind,
        FileId,
    },
    sync::Arc,
    util::CachedString,
};

#[derive(Debug)]
pub struct CTokenStack {
    tokens: Vec<CToken>,
    file_references: HashMap<CachedString, Option<FileId>>,
    errors: Vec<CLexerError>,
    path: Option<Arc<Path>>,
    file_id: FileId,
}
// The stack should always contain at least one token (EOF).
#[allow(clippy::len_without_is_empty)]
impl CTokenStack {
    pub fn new(file_id: FileId, path: Option<Arc<Path>>) -> Self {
        CTokenStack {
            tokens: Vec::new(),
            file_references: HashMap::new(),
            errors: Vec::new(),
            file_id,
            path,
        }
    }

    pub fn new_empty(file_id: FileId, path: Option<Arc<Path>>) -> Self {
        let mut this = CTokenStack::new(file_id, path);
        this.append(CToken::new_first_byte(file_id, CTokenKind::Eof));
        this.finalize();
        this
    }

    pub fn new_error(file_id: FileId, path: Option<Arc<Path>>, error: CLexerError) -> Self {
        let mut this = CTokenStack::new(file_id, path);
        this.add_error_token(error);
        this.append(CToken::new_first_byte(file_id, CTokenKind::Eof));
        this.finalize();
        this
    }

    pub fn append(&mut self, token: CToken) -> usize {
        let index = self.tokens.len();
        self.tokens.push(token);
        index
    }

    pub fn add_reference(&mut self, include_name: &CachedString, file_id: Option<FileId>) {
        self.file_references.insert(include_name.clone(), file_id);
    }

    pub fn add_error_token(&mut self, error: CLexerError) {
        let index = self.errors.len();
        self.errors.push(error);
        // TODO: Add it with a location.
        let error_token = CToken::new_unknown(CTokenKind::LexerError(index));
        self.append(error_token);
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn get_file_ref(&self, inc_str: &CachedString) -> Option<FileId> {
        *self.file_references.get(inc_str)?
    }

    pub fn errors(&self) -> &Vec<CLexerError> {
        &self.errors
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn finalize(&mut self) {
        let difference = self.tokens.capacity() - self.tokens.len();
        if difference > 100 {
            self.tokens.shrink_to_fit();
        }
    }
}
impl std::ops::Index<usize> for CTokenStack {
    type Output = CToken;

    fn index(&self, index: usize) -> &CToken {
        &self.tokens[index]
    }
}
impl std::ops::IndexMut<usize> for CTokenStack {
    fn index_mut(&mut self, index: usize) -> &mut CToken {
        &mut self.tokens[index]
    }
}

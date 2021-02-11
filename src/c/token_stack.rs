// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::collections::HashMap;
use std::path::Path;

use crate::{
    c::{
        CToken,
        FileId,
    },
    sync::Arc,
    util::CachedString,
};

#[derive(Debug)]
pub struct CTokenStack {
    tokens: Vec<CToken>,
    file_references: HashMap<CachedString, Option<FileId>>,
    file_path: Option<Arc<Path>>,
    file_id: FileId,
}
// The stack should always contain at least one token (EOF).
#[allow(clippy::len_without_is_empty)]
impl CTokenStack {
    pub fn new(file_id: FileId, path: &Option<Arc<Path>>) -> Self {
        CTokenStack {
            tokens: Vec::new(),
            file_references: HashMap::new(),
            file_id,
            file_path: path.clone(),
        }
    }

    pub fn append(&mut self, token: CToken) -> usize {
        self.tokens.push(token);
        self.tokens.len() - 1
    }

    pub fn add_reference(&mut self, include_name: &CachedString, file_id: Option<FileId>) {
        self.file_references.insert(include_name.clone(), file_id);
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

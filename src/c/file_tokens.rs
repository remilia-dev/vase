// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::{
    collections::HashMap,
    path::Path,
};

use crate::{
    c::{
        LexerError,
        LexerErrorKind,
        Token,
        TokenKind,
    },
    sync::Arc,
    util::{
        CachedString,
        FileId,
        SourceLocation,
    },
};

#[derive(Debug)]
pub struct FileTokens {
    tokens: Vec<Token>,
    file_references: HashMap<CachedString, Option<FileId>>,
    errors: Vec<LexerError>,
    path: Option<Arc<Path>>,
    file_id: FileId,
}
// The stack should always contain at least one token (EOF).
#[allow(clippy::len_without_is_empty)]
impl FileTokens {
    pub fn new(file_id: FileId, path: Option<Arc<Path>>) -> Self {
        FileTokens {
            tokens: Vec::new(),
            file_references: HashMap::new(),
            errors: Vec::new(),
            file_id,
            path,
        }
    }

    pub fn new_empty(file_id: FileId, path: Option<Arc<Path>>) -> Self {
        let mut this = FileTokens::new(file_id, path);
        this.append(Token::new_first_byte(file_id, TokenKind::Eof));
        this.finalize();
        this
    }

    pub fn new_error<T>(file_id: FileId, path: Option<Arc<Path>>, error: T) -> Self
    where T: Into<LexerErrorKind> {
        let mut this = FileTokens::new(file_id, path);
        let location = SourceLocation::new_first_byte(file_id);
        this.add_error_token(LexerError { location, kind: error.into() });
        this.append(Token::new_first_byte(file_id, TokenKind::Eof));
        this.finalize();
        this
    }

    pub fn append(&mut self, token: Token) -> usize {
        let index = self.tokens.len();
        self.tokens.push(token);
        index
    }

    pub fn add_reference(&mut self, include_name: &CachedString, file_id: Option<FileId>) {
        self.file_references.insert(include_name.clone(), file_id);
    }

    pub fn add_error_token(&mut self, error: LexerError) {
        let index = self.errors.len();
        let location = error.location.clone();
        self.errors.push(error);
        let error_token = Token::new(location, false, TokenKind::LexerError(index));
        self.append(error_token);
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn file_id(&self) -> FileId {
        self.file_id
    }

    pub fn path(&self) -> &Option<Arc<Path>> {
        &self.path
    }

    pub fn get_file_ref(&self, inc_str: &CachedString) -> Option<FileId> {
        *self.file_references.get(inc_str)?
    }

    pub fn errors(&self) -> &Vec<LexerError> {
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
impl std::ops::Index<usize> for FileTokens {
    type Output = Token;

    fn index(&self, index: usize) -> &Token {
        &self.tokens[index]
    }
}
impl std::ops::IndexMut<usize> for FileTokens {
    fn index_mut(&mut self, index: usize) -> &mut Token {
        &mut self.tokens[index]
    }
}

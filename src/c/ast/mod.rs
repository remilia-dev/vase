// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use decl::{
    Decl,
    DeclIndex,
    DeclPostfix,
    DeclRefExpr,
    DeclStmt,
};
pub use expr::*;
pub use file::SourceFile;
pub use number::{
    Number,
    NumberError,
    NumberKind,
};
pub use operators::{
    AssignOp,
    Associativity,
    BinaryOp,
    Precedence,
    PrefixOp,
    SuffixOp,
    TypeOp,
};
pub use qualifications::{
    Storage,
    StorageKind,
};
pub use scope::{
    Scope,
    ScopeId,
    ScopeKind,
};
pub use stmt::*;
pub use string::StringLiteral;
pub use types::*;
pub use visitor::Visitor;

mod decl;
mod expr;
mod file;
mod number;
mod operators;
mod qualifications;
mod scope;
mod stmt;
mod string;
mod types;
mod visitor;

#[derive(Clone, Debug)]
pub struct Id {
    pub text: crate::util::CachedString,
    pub index: crate::c::TravelIndex,
}

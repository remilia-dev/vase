// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use compile_env::CompileEnv;
pub use file_reader::FileReader;
pub use file_tokens::FileTokens;
pub use lexer::Lexer;
pub use lexer_error::{
    LexerError,
    LexerErrorKind,
};
#[cfg(all(feature = "file-reading", feature = "multithreading"))]
pub use multi_lexer::MultiLexer;
pub use parser::{
    ParseError,
    ParseErrorKind,
    Parser,
};
pub use settings::*;
pub use token::*;
pub use traveler::*;

pub mod ast;
mod compile_env;
mod file_reader;
mod file_tokens;
mod lexer;
mod lexer_error;
#[cfg(all(feature = "file-reading", feature = "multithreading"))]
mod multi_lexer;
mod parser;
mod settings;
mod token;
mod traveler;

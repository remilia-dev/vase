// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use compile_env::CompileEnv;
pub use error::*;
pub use file_reader::FileReader;
pub use file_tokens::FileTokens;
pub use lexer::Lexer;
pub use multi_lexer::MultiLexer;
pub use settings::*;
pub use token::*;
pub use traveler::*;

pub type FileId = u32;

mod compile_env;
mod error;
mod file_reader;
mod file_tokens;
mod lexer;
mod multi_lexer;
mod settings;
mod token;
mod traveler;

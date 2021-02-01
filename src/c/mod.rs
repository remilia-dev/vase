// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use compile_env::CCompileEnv;
pub use file_reader::*;
pub use lexer::*;
pub use multi_lexer::*;
pub use settings::*;
pub use token::*;
pub use token_stack::CTokenStack;

pub type FileId = u32;

mod compile_env;
mod file_reader;
mod lexer;
mod multi_lexer;
mod settings;
mod token;
mod token_stack;

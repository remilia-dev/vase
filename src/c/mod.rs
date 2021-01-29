// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
pub use char_reader::*;
pub use compile_env::CCompileEnv;
pub use lexer::*;
pub use multi_lexer::*;
pub use settings::*;
pub use token::*;
pub use token_stack::CTokenStack;

pub type FileId = u32;

mod char_reader;
mod compile_env;
mod lexer;
mod multi_lexer;
mod settings;
mod token;
mod token_stack;

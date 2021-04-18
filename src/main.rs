// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use vase::{
    c::*,
    sync::Arc,
};

fn main() {
    let mut settings = CompileSettings::default();
    settings.source_files.push(Arc::from(Path::new("./test.c")));
    let env = Arc::new(CompileEnv::new(settings));
    let mut lexer = MultiLexer::new(env.clone());
    lexer.lex_multi_threaded(&*env.settings().source_files);

    let mut parser = Parser::new(&env, |error: ParseError| {
        panic!("{:?}", error);
        //false
    });
    let tokens = env.file_id_to_tokens.get_arc(0.into()).unwrap();
    println!("Tokens In File: {}", tokens.len());
    let parsed = parser.parse(tokens).unwrap();
    println!("{:#?}", parsed)
}

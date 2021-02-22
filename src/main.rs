// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::path::Path;

use vase::c::*;
use vase::sync::Arc;

fn main() {
    let mut settings = CompileSettings::default();
    settings.source_files.push(Arc::from(Path::new("./test.c")));
    let env = Arc::new(CompileEnv::new(settings));
    let mut lexer = MultiLexer::new(env.clone());
    lexer.lex_multi_threaded(&*env.settings().source_files);

    let mut traveler = Traveler::new(env.clone());
    let tokens = env.file_id_to_tokens()[0].clone();
    println!("{:#?}", tokens);
    traveler.load_start(tokens);

    let mut tokens = Vec::new();
    loop {
        match traveler.head().kind() {
            TokenKind::Eof => break,
            token => {
                tokens.push(token.clone());
                traveler.move_forward();
            },
        }
    }

    println!("{:#?}", tokens);
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
mod comments;
mod preprocessor;
mod symbols;

use std::path::Path;

use vase::{
    c::{
        CompileEnv,
        Lexer,
        TokenKind,
    },
    sync::Arc,
    util::{
        CachedString,
        FileId,
    },
};

fn run_test(env: &CompileEnv, source: &str, expected: &[TokenKind], allow_includes: bool) {
    let callback = &|_, _: &CachedString, _: &Option<Arc<Path>>| -> Option<FileId> {
        assert!(
            allow_includes,
            "Include occurred in a test that does not allow includes."
        );
        None
    };
    let mut lexer = Lexer::new(&env, callback);
    let tokens = lexer.lex_bytes(0.into(), source.as_bytes());

    for i in 0..expected.len() {
        assert_eq!(tokens[i].kind(), &expected[i], "Index: {}", i);
    }
}

#[test]
fn escape_new_line_adds_to_token_length() {
    let env = CompileEnv::default();
    let callback = |_, _: &CachedString, _: &Option<Arc<Path>>| panic!("No includes should occur!");
    let mut lexer = Lexer::new(&env, callback);
    let tokens = lexer.lex_bytes(0.into(), "+\\\n=\\\n+=+=\\\n".as_bytes());
    // The escape-newline is included in the length of the token if it occurs in the center.
    assert_eq!(tokens[0].loc().byte_length, 4);
    // The escape-newline is not included in the length of the token if it is at the start or ending.
    assert_eq!(tokens[1].loc().byte_length, 2);
    assert_eq!(tokens[2].loc().byte_length, 2);
}

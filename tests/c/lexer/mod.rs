// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
mod comments;
mod preprocessor;
mod symbols;

use std::path::Path;

use vase::{
    c::{
        CCompileEnv,
        CCompileSettings,
        CLexer,
        CTokenKind,
        FileId,
    },
    sync::Arc,
    util::CachedString,
};

fn new_env() -> Arc<CCompileEnv> {
    Arc::new(CCompileEnv::new(CCompileSettings::default()))
}

fn run_test(env: Arc<CCompileEnv>, source: &str, expected: &[CTokenKind], allow_includes: bool) {
    let callback = &|_, _: &CachedString, _: &Option<Arc<Path>>| -> Option<FileId> {
        assert!(
            allow_includes,
            "Include occurred in a test that does not allow includes."
        );
        None
    };
    let mut lexer = CLexer::new(&env, callback);
    let tokens = lexer.lex_bytes(0, source.as_bytes()).unwrap();

    for i in 0..expected.len() {
        assert_eq!(tokens[i].kind(), &expected[i], "Index: {}", i);
    }
}

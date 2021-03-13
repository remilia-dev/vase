// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
mod conditional;
mod include;
mod macros;
mod token_joining;

use std::path::Path;

use vase::{
    c::{
        CompileEnv,
        CompileSettings,
        Lexer,
        TokenKind,
        Traveler,
        TravelerError,
    },
    error::CodedError,
    math::NonMaxU32,
    sync::Arc,
    util::CachedString,
};

fn new_env() -> Arc<CompileEnv> {
    Arc::new(CompileEnv::new(CompileSettings::default()))
}

fn run_test(env: Arc<CompileEnv>, sources: &[&str], expected: &[TokenKind]) {
    if sources.len() > 2 {
        panic!(
            "This test helper can only support up to two sources. All includes go to the second source."
        );
    }

    let callback = |_, _: &CachedString, _: &Option<Arc<Path>>| Some(1.into());
    let mut lexer = Lexer::new(&env, callback);
    for (i, source) in sources.iter().enumerate() {
        let file_id = NonMaxU32::new(i as u32).unwrap();
        let tokens = lexer.lex_bytes(file_id, source.as_bytes());
        env.file_id_to_tokens().push(Arc::new(tokens));
    }

    let mut traveler = Traveler::new(env.clone(), &|err: TravelerError| {
        panic!(
            "An error should not have occured: {:?}\n{}",
            &err,
            err.message()
        );
    });
    traveler
        .load_start(env.file_id_to_tokens()[0.into()].clone())
        .unwrap();

    for expected_token in expected.iter() {
        assert_eq!(traveler.head().kind(), expected_token);
        traveler.move_forward().unwrap();
    }

    assert_eq!(traveler.head().kind(), &TokenKind::Eof);
}

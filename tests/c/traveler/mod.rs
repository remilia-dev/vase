// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
mod conditional;
mod include;
mod macros;
mod token_joining;

use vase::{
    c::{
        CompileEnv,
        CompileSettings,
        Lexer,
        TokenKind,
        Traveler,
    },
    sync::Arc,
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

    let mut lexer = Lexer::new(&env, &|_, _, _| Some(1));
    for (i, source) in sources.iter().enumerate() {
        let tokens = lexer.lex_bytes(i as u32, source.as_bytes());
        env.file_id_to_tokens().push(Arc::new(tokens));
    }

    let mut traveler = Traveler::new(env.clone(), &|err| {
        panic!("An error should not have occured: {:?}", err);
    });
    traveler.load_start(env.file_id_to_tokens()[0].clone()).unwrap();

    for expected_token in expected.iter() {
        assert_eq!(traveler.head().kind(), expected_token);
        traveler.move_forward().unwrap();
    }

    assert_eq!(traveler.head().kind(), &TokenKind::Eof);
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
mod conditional;
mod include;
mod macros;
mod token_joining;

use vase::{
    c::{
        CCompileEnv,
        CCompileSettings,
        CLexer,
        CTokenKind,
        CTraveler,
    },
    sync::Arc,
};

fn new_env() -> Arc<CCompileEnv> {
    Arc::new(CCompileEnv::new(CCompileSettings::default()))
}

fn run_test(env: Arc<CCompileEnv>, sources: &[&str], expected: &[CTokenKind]) {
    if sources.len() > 2 {
        panic!(
            "This test helper can only support up to two sources. All includes go to the second source."
        );
    }

    let mut lexer = CLexer::new(&env, &|_, _, _| Some(1));
    for (i, source) in sources.iter().enumerate() {
        let tokens = lexer.lex_bytes(i as u32, source.as_bytes());
        env.file_id_to_tokens().push(Arc::new(tokens));
    }

    let mut traveler = CTraveler::new(env.clone());
    traveler.load_start(env.file_id_to_tokens()[0].clone());

    for expected_token in expected.iter() {
        assert_eq!(traveler.head().kind(), expected_token);
        traveler.move_forward();
    }

    assert_eq!(traveler.head().kind(), &CTokenKind::Eof);
}

// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::c::{
    CCompileEnv,
    CCompileSettings,
    CLexer,
    CTokenKind,
    CTokenKind::*,
};

#[test]
fn test_comment_lexing() {
    let env = CCompileEnv::new(CCompileSettings::default());
    let mut lexer = CLexer::new(&env, &|_, _, _| panic!("No includes should occur!"));
    let tokens = lexer.lex_bytes(0, TEST_CASE.as_bytes()).unwrap();
    for i in 0..tokens.len() {
        assert_eq!(*tokens[i].kind(), TEST_RESULT[i], "At index: {}", i);
    }
}

static TEST_CASE: &'static str = r#"
// NOTE: I used a minus token (+) to separate comments

// This should not be a token \
and neither should it when continued on a line
+
/\
/ You can split up the two slashes
+
// */ nothing ends a //-comment except a new-line
+
/* This
should
* not
be a token*/ +

/\
* Can continue symbols across lines *\
/ +

/* /* does not nest */ */
"#;

// The 5 pluses separate comments. The star and slash test that multi-line comments do not nest.
static TEST_RESULT: &'static [CTokenKind] = &[Plus, Plus, Plus, Plus, Plus, Star, Slash, EOF];

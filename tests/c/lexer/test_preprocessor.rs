// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::{
    c::{
        CCompileEnv,
        CCompileSettings,
        CIncludeType::*,
        CLexer,
        CTokenKind,
        CTokenKind::*,
    },
    sync::Arc,
    util::StringCache,
};

#[test]
fn test_preprocessor_lexing() {
    let env = CCompileEnv::new(CCompileSettings::default());
    let mut lexer = CLexer::new(&env, &|_, _, _| -> u32 { 1 });
    let tokens = lexer.lex_bytes(0, TEST_CASE.as_bytes()).unwrap();
    let test_results = test_results(env.cache());
    for i in 0..tokens.len() {
        assert_eq!(*tokens[i].kind(), test_results[i], "At index: {}", i);
    }
}

static TEST_CASE: &'static str = r#"
#define
// Ensure many spaces can be used
#                                define
// Even escape new-lines
#  \
define

// Test lexing and proper linking:
#ifdef
#ifndef
#elif
#else
#endif
#endif

// Test lexing:
#define
#undef
#line
#pragma
#

// Include-mode-based:
#include "an include"
#include <a sys include>
#include_next "a next include"
#include_next <also a next include>

// Message-mode-based:
#error
#error An error message
#warning A warning message
#warning can \
span \
lines

# not_in_the_standard
"#;

fn test_results(cache: &StringCache) -> Box<[CTokenKind]> {
    Box::from([
        PreDefine, PreEnd,
        PreDefine, PreEnd,
        PreDefine, PreEnd,

        PreIfDef { link: 16 }, PreEnd,
        PreIfNDef { link: 10 }, PreEnd,
        PreElif { link: 12 }, PreEnd,
        PreElse { link: 14 }, PreEnd,
        PreEndIf, PreEnd,
        PreEndIf, PreEnd,

        PreDefine, PreEnd,
        PreUndef, PreEnd,
        PreLine, PreEnd,
        PrePragma, PreEnd,
        PreBlank,

        PreInclude, IncludePath {
            inc_type: IncludeLocal,
            path: cache.get_or_cache("an include"),
        }, PreEnd,
        PreInclude, IncludePath {
            inc_type: IncludeSystem,
            path: cache.get_or_cache("a sys include"),
        }, PreEnd,
        PreIncludeNext, IncludePath {
            inc_type: IncludeNext,
            path: cache.get_or_cache("a next include"),
        }, PreEnd,
        PreIncludeNext, IncludePath {
            inc_type: IncludeNext,
            path: cache.get_or_cache("also a next include"),
        }, PreEnd,

        PreError, PreEnd,
        PreError, Message(Arc::new(Box::from("An error message"))),
        PreWarning, Message(Arc::new(Box::from("A warning message"))),
        PreWarning, Message(Arc::new(Box::from("can span lines"))),

        PreUnknown(cache.get_or_cache("not_in_the_standard")), PreEnd,
        EOF,
    ])
}

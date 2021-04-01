// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use indoc::indoc;
use vase::{
    c::{
        CompileEnv,
        IncludeType::*,
        TokenKind::*,
    },
    sync::Arc,
};

use super::run_test;

#[test]
fn preprocessor_tokens_lex_properly() {
    let env = CompileEnv::default();
    let cache = env.cache();
    run_test(
        &env,
        indoc! {r#"
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
        "#},
        &[
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
            Eof,
        ],
        true,
    );
}

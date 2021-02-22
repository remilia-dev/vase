// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::c::TokenKind::*;

use super::{
    new_env,
    run_test,
};

#[test]
fn includes_work() {
    let env = new_env();
    let cache = env.cache();
    let expected = [
        Identifier(cache.get_or_cache("includes_work")),
        Identifier(cache.get_or_cache("include_macro_works")),
    ];
    run_test(
        env,
        &[
            r#"
            // This include will link to the second source:
            #include "the include doesn't matter"
            MACRO_FROM_INCLUDE
            "#,
            r#"
            #define MACRO_FROM_INCLUDE include_macro_works
            includes_work
            "#,
        ],
        &expected,
    );
}

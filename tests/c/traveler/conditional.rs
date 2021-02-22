// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::c::TokenKind::*;

use super::{
    new_env,
    run_test,
};

#[test]
fn preprocessor_ifdef_works() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define DEFINED

        #ifdef UNDEFINED
            Should not occur
        #endif

        #ifdef DEFINED
            ifdef_works
        #endif

        #ifndef UNDEFINED
            ifndef_works
        #endif

        #ifndef DEFINED
            Should not occcur
        #endif
        "#],
        &[
            Identifier(cache.get_or_cache("ifdef_works")),
            Identifier(cache.get_or_cache("ifndef_works")),
        ],
    );
}

#[test]
fn preprocessor_else_works() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define DEFINED

        #ifdef UNDEFINED
            // Empty to check that empty ifs work
        #else
            else_ifdef_works
        #endif

        #ifdef DEFINED
            ifdef_works
        #else
            Should be skipped
        #endif

        #ifndef UNDEFINED
            ifndef_works
        #else
            Should be skipped
        #endif

        #ifndef DEFINED
            Should not occcur
        #else
            else_ifndef_works
        #endif

        #ifndef DEFINED
        #else
            // Empty to check that empty elses work
        #endif
        "#],
        &[
            Identifier(cache.get_or_cache("else_ifdef_works")),
            Identifier(cache.get_or_cache("ifdef_works")),
            Identifier(cache.get_or_cache("ifndef_works")),
            Identifier(cache.get_or_cache("else_ifndef_works")),
        ],
    );
}

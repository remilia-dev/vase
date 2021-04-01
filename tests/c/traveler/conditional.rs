// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::c::{
    CompileEnv,
    TokenKind::*,
};

use super::run_test;

#[test]
fn preprocessor_ifdef_works() {
    let env = CompileEnv::default();
    let cache = env.cache();
    run_test(
        &env,
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
    let env = CompileEnv::default();
    let cache = env.cache();
    run_test(
        &env,
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

#[test]
fn preprocessor_if_conditions_work() {
    let env = CompileEnv::default();
    let cache = env.cache();
    run_test(
        &env,
        &[r#"
        #if 0
            IsFalse
        #endif

        #if 1
            IsTrue
        #endif

        #if 7 == 1 + 2 * 3
            PrecedenceWorks
        #endif

        #if 1 ? 0 : 1
            TernaryIsBackwards
        #elif 1 ? 1 : 0
            TernaryWorks
        #endif

        #define EMPTY
        #if 0 + EMPTY 1
            ReplacementOccurs
        #endif

        #if (1 - 1)
            #error Should never occur
        #endif

        #if UNDEFINED == 0
            UndefinedReplacedWith0
        #endif
        "#],
        &[
            Identifier(cache.get_or_cache("IsTrue")),
            Identifier(cache.get_or_cache("PrecedenceWorks")),
            Identifier(cache.get_or_cache("TernaryWorks")),
            Identifier(cache.get_or_cache("ReplacementOccurs")),
            Identifier(cache.get_or_cache("UndefinedReplacedWith0")),
        ],
    );
}

#[test]
fn preprocessor_if_char_literals_work() {
    let env = CompileEnv::default();
    let cache = env.cache();
    run_test(
        &env,
        &[r#"
        #if '\0' || '\00' || '\000'
            IsFalse
        #endif

        #if 'z' - 'a' == 25
            IsTrue
        #endif

        #if u'\uFFFF' == 0xFFFF
            UnicodeWorks
        #endif
        "#],
        &[
            Identifier(cache.get_or_cache("IsTrue")),
            Identifier(cache.get_or_cache("UnicodeWorks")),
        ],
    );
}

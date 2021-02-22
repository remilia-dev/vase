// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::c::Keyword;
use vase::c::TokenKind::*;

use super::{
    new_env,
    run_test,
};

#[test]
fn object_macros_work() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define EMPTY // Empty object macro shouldn't produce tokens
        EMPTY

        #define SINGLE single_macro
        SINGLE

        #define MULTIPLE multiple tokens
        MULTIPLE

        #define int long
        int

        #define X1 X2
        #define X2 X3 // Can chain
        X1

        #define RECURSE RECURSE // not recursive.
        RECURSE
        "#],
        &[
            Identifier(cache.get_or_cache("single_macro")),
            Identifier(cache.get_or_cache("multiple")),
            Identifier(cache.get_or_cache("tokens")),
            Keyword(Keyword::Long, cache.get_or_cache("long").uniq_id()),
            Identifier(cache.get_or_cache("X3")),
            Identifier(cache.get_or_cache("RECURSE")),
        ],
    );
}

#[test]
fn function_macro_var_args_work() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define NO_ARG(...) __VA_ARGS__
        NO_ARG(NA1, NA2)

        #define ONE_ARG(A, ...) __VA_ARGS__
        ONE_ARG(OA1, OA2, OA3)

        #define NAMED_VAR_ARG(var_args...) var_args, __VA_ARG__
        NAMED_VAR_ARG(NVA1, NVA2)
        "#],
        &[
            // NO_ARG(NA1, NA2) produces:
            Identifier(cache.get_or_cache("NA1")),
            Comma,
            Identifier(cache.get_or_cache("NA2")),
            // ONE_ARG(OA1, OA2, OA3) produces:
            Identifier(cache.get_or_cache("OA2")),
            Comma,
            Identifier(cache.get_or_cache("OA3")),
            // NAMED_VAR_ARG(NVA1, NVA2) produces:
            Identifier(cache.get_or_cache("NVA1")),
            Comma,
            Identifier(cache.get_or_cache("NVA2")),
            Comma,
            Identifier(cache.get_or_cache("__VA_ARG__")),
        ],
    );
}

#[test]
fn partial_function_macro_invocations_work() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define X() Y
        #define Y X(

        // 1. Y expands this to X())
        // 2. X() expands this to Y)
        // 3. Y expands this to X()
        // 4. We're still inside step 2, which is an invocation of X(). So we treat the X as a normal X.
        Y))
        "#],
        &[Identifier(cache.get_or_cache("X")), LParen, RParen],
    );
}

#[test]
fn indirect_function_invocations_work() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define EMPTY
        #define EMPTY_INDIRECT EMPTY
        #define EMPTY_FUNC() EMPTY
        #define LP (
        #define RP )
        #define X() expected_result
        #define Y(EMPTY_TOKENS, EMPTY_ARG, L, R) X EMPTY_TOKENS EMPTY_ARG L R

        Y(EMPTY EMPTY_INDIRECT EMPTY_FUNC(), , LP, RP)
        "#],
        &[Identifier(cache.get_or_cache("expected_result"))],
    );
}

#[test]
fn can_undef_macros() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define OBJECT DEFINED
        #undef OBJECT
        OBJECT

        #define FUNCTION() DEFINED
        #undef FUNCTION
        FUNCTION()

        #define int long
        #undef int
        int
        "#],
        &[
            Identifier(cache.get_or_cache("OBJECT")),
            Identifier(cache.get_or_cache("FUNCTION")),
            LParen,
            RParen,
            Keyword(Keyword::Int, cache.get_or_cache("int").uniq_id()),
        ],
    );
}

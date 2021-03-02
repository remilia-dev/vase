// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use vase::{
    c::{
        StringEncoding,
        TokenKind::*,
    },
    sync::Arc,
};

use super::{
    new_env,
    run_test,
};

#[test]
fn can_join_symbols() {
    run_test(
        new_env(),
        &[r#"
        #define JOIN(A, B) A ## B
        JOIN(<,:)
        JOIN(:,>)
        JOIN(<,%)
        JOIN(%,>)
        JOIN(&,=)
        JOIN(&,&)
        JOIN(-,>)
        JOIN(!,=)
        JOIN(|,=)
        JOIN(|,|)
        JOIN(^,=)
        JOIN(=,=)
        // We can't really test the # and ## combinations as they are handled by the traveler
        JOIN(-,=)
        JOIN(-,-)
        JOIN(<,=)
        JOIN(<,<)
        JOIN(<,<=)
        JOIN(<<,=)
        JOIN(%,=)
        JOIN(+,=)
        JOIN(+,+)
        JOIN(>,=)
        JOIN(>,>)
        JOIN(>,>=)
        JOIN(>>,=)
        JOIN(/,=)
        JOIN(*,=)
        "#],
        &[
            LBracket { alt: true },
            RBracket { alt: true },
            LBrace { alt: true },
            RBrace { alt: true },
            AmpEqual,
            AmpAmp,
            Arrow,
            BangEqual,
            BarEqual,
            BarBar,
            CarrotEqual,
            EqualEqual,
            MinusEqual,
            MinusMinus,
            LAngleEqual,
            LShift,
            LShiftEqual,
            LShiftEqual,
            PercentEqual,
            PlusEqual,
            PlusPlus,
            RAngleEqual,
            RShift,
            RShiftEqual,
            RShiftEqual,
            SlashEqual,
            StarEqual,
        ],
    );
}

#[test]
fn can_join_str_prefix() {
    let expected: Arc<Box<str>> = Arc::new(Box::from("test"));
    run_test(
        new_env(),
        &[r#"
        #define TEST "test"
        #define JOIN(A, B) A ## B
        TEST
        JOIN(u8, TEST)
        JOIN(u, TEST)
        JOIN(U, TEST)
        JOIN(L, TEST)
        "#],
        &[
            String {
                encoding: StringEncoding::Default,
                str_data: expected.clone(),
                has_escapes: false,
                is_char: false,
            },
            String {
                encoding: StringEncoding::U8,
                str_data: expected.clone(),
                has_escapes: false,
                is_char: false,
            },
            String {
                encoding: StringEncoding::U16,
                str_data: expected.clone(),
                has_escapes: false,
                is_char: false,
            },
            String {
                encoding: StringEncoding::U32,
                str_data: expected.clone(),
                has_escapes: false,
                is_char: false,
            },
            String {
                encoding: StringEncoding::WChar,
                str_data: expected,
                has_escapes: false,
                is_char: false,
            },
        ],
    );
}

#[test]
fn can_join_numbers() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define JOIN(A, B) A ## B
        JOIN(0x, FF)
        JOIN(0b, 2)
        // So long as something starts with a number, it is considered one
        JOIN(0, some_random_identifier)
        // Dots can be joined
        JOIN(1, .)
        JOIN(., 1)

        #define JOIN_CHAIN(A, B, C) A ## B ## C
        // If the number ends in an exponent, pasting with +/- is allowed
        JOIN_CHAIN(1E, +, 2)
        JOIN_CHAIN(0x1P, -, 2)
        // Dots and numbers can of course be joined to form a float literal:
        JOIN_CHAIN(0xF, ., C)
        "#],
        &[
            Number(cache.get_or_cache("0xFF")),
            Number(cache.get_or_cache("0b2")),
            Number(cache.get_or_cache("0some_random_identifier")),
            Number(cache.get_or_cache("1.")),
            Number(cache.get_or_cache(".1")),
            Number(cache.get_or_cache("1E+2")),
            Number(cache.get_or_cache("0x1P-2")),
            Number(cache.get_or_cache("0xF.C")),
        ],
    );
}

#[test]
fn can_join_identifiers() {
    let env = new_env();
    let cache = env.cache();
    run_test(
        env.clone(),
        &[r#"
        #define JOIN(A, B) A ## B
        JOIN(A, B)
        JOIN(Rust, y)
        JOIN(C, 4)
        // Joining should be able to chain
        #define JOIN_CHAIN(A, B, C) A ## B ## C
        JOIN_CHAIN(X, Y, Z)
        // Can't joint to invoke a function macro argument
        #define NO_INDIRECT_ARG(AB) A ## B
        NO_INDIRECT_ARG(unexpected)
        // You can join to invoke another macro
        #define INDIRECT_MACRO JO ## IN (exp, ected)
        INDIRECT_MACRO
        "#],
        &[
            Identifier(cache.get_or_cache("AB")),
            Identifier(cache.get_or_cache("Rusty")),
            Identifier(cache.get_or_cache("C4")),
            Identifier(cache.get_or_cache("XYZ")),
            Identifier(cache.get_or_cache("AB")),
            Identifier(cache.get_or_cache("expected")),
        ],
    );
}

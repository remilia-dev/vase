// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use indoc::indoc;
use vase::c::TokenKind::*;

use super::{
    new_env,
    run_test,
};

#[test]
fn comments_lex_properly() {
    run_test(
        new_env(),
        indoc! {r#"
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
        "#},
        // The 5 pluses separate comments. The star and slash test that multi-line comments do not nest.
        &[Plus, Plus, Plus, Plus, Plus, Star, Slash, Eof],
        false,
    );
}

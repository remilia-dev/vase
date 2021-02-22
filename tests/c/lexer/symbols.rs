// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use indoc::indoc;
use vase::c::TokenKind::*;

use super::{
    new_env,
    run_test,
};

#[test]
fn test_symbol_lexing() {
    run_test(
        new_env(),
        indoc! {r#"
        [
        <:
        <\
        :

        ]
        :>
        :\
        >

        (

        )

        {
        <%
        <\
        %

        }
        %>
        %\
        >

        &

        &=
        &\
        =

        &&
        &\
        &

        ->
        -\
        >

        @

        \ // Comment necessary for this to be lexed as a symbol

        !

        !=
        !\
        =

        |

        |=
        |\
        =

        ||
        |\
        |

        ^

        ^=
        ^\
        =

        :

        ,

        .

        ...
        .\
        ..
        ..\
        .
        .\
        .\
        .

        =

        ==
        =\
        =

        // Pluses are to avoid it lexing a preprocessor instruction
        + #
        + %:
        + %\
        :

        ##
        #\
        #
        %:%:
        %\
        :%:
        %:\
        %:
        %:%\
        :
        %\
        :\
        %:
        %:\
        %\
        :
        %\
        :%\
        :
        %\
        :\
        %\
        :

        -

        -=
        -\
        =

        --
        -\
        -

        <

        <=
        <\
        =

        <<
        <\
        <

        <<=
        <\
        <=
        <<\
        =
        <\
        <\
        =

        %

        %=
        %\
        =

        +

        +=
        +\
        =

        ++
        +\
        +

        ?

        >

        >=
        >\
        =

        >>
        >\
        >

        >>=
        >\
        >=
        >>\
        =
        >\
        >\
        =

        ;

        /

        /=
        /\
        =

        *

        *=
        *\
        =

        ~
        "#},
        &[
            LBracket { alt: false },
            LBracket { alt: true },
            LBracket { alt: true },

            RBracket { alt: false },
            RBracket { alt: true },
            RBracket { alt: true },

            LParen,

            RParen,

            LBrace { alt: false },
            LBrace { alt: true },
            LBrace { alt: true },

            RBrace { alt: false },
            RBrace { alt: true },
            RBrace { alt: true },

            Amp,

            AmpEqual,
            AmpEqual,

            AmpAmp,
            AmpAmp,

            Arrow,
            Arrow,

            At,

            Backslash,

            Bang,

            BangEqual,
            BangEqual,

            Bar,

            BarEqual,
            BarEqual,

            BarBar,
            BarBar,

            Carrot,

            CarrotEqual,
            CarrotEqual,

            Colon,

            Comma,

            Dot,

            DotDotDot,
            DotDotDot,
            DotDotDot,
            DotDotDot,

            Equal,

            EqualEqual,
            EqualEqual,

            Plus, Hash { alt: false },
            Plus, Hash { alt: true },
            Plus, Hash { alt: true },

            HashHash { alt: false },
            HashHash { alt: false },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },
            HashHash { alt: true },

            Minus,

            MinusEqual,
            MinusEqual,

            MinusMinus,
            MinusMinus,

            LAngle,

            LAngleEqual,
            LAngleEqual,

            LShift,
            LShift,

            LShiftEqual,
            LShiftEqual,
            LShiftEqual,
            LShiftEqual,

            Percent,

            PercentEqual,
            PercentEqual,

            Plus,

            PlusEqual,
            PlusEqual,

            PlusPlus,
            PlusPlus,

            QMark,

            RAngle,

            RAngleEqual,
            RAngleEqual,

            RShift,
            RShift,

            RShiftEqual,
            RShiftEqual,
            RShiftEqual,
            RShiftEqual,

            Semicolon,

            Slash,

            SlashEqual,
            SlashEqual,

            Star,

            StarEqual,
            StarEqual,

            Tilde,
            Eof,
        ],
        false,
    );
}

use super::Lexer;
use crate::{ast, SourceId};

macro_rules! test_lexer {
    ($source:expr $(, $pat:pat)* $(,)?) => {{
        let mut it = Lexer::new($source, SourceId::empty(), false);

        #[allow(unused_assignments)]
        {
            let mut n = 0;

            $(
                match it.next().unwrap().expect("expected token") {
                    $pat => (),
                    #[allow(unreachable_patterns)]
                    other => {
                        panic!("\nGot bad token #{}.\nExpected: `{}`\nBut got: {:?}", n, stringify!($pat), other);
                    }
                }

                n += 1;
            )*
        }

        assert_eq!(it.next().unwrap(), None);
    }}
}

#[test]
fn test_number_literals() {
    test_lexer! {
        "(10)",
        ast::Token {
            span: span!(0, 1),
            kind: ast::Kind::Open(ast::Delimiter::Parenthesis),
        },
        ast::Token {
            span: span!(1, 3),
            kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                source_id: SourceId::EMPTY,
                is_fractional: false,
                base: ast::NumberBase::Decimal,
                number: span!(1, 3),
                suffix: span!(3, 3),
            })),
        },
        ast::Token {
            span: span!(3, 4),
            kind: ast::Kind::Close(ast::Delimiter::Parenthesis),
        },
    };

    test_lexer! {
        "(10.)",
        _,
        ast::Token {
            span: span!(1, 4),
            kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                source_id: SourceId::EMPTY,
                is_fractional: true,
                base: ast::NumberBase::Decimal,
                number: span!(1, 4),
                suffix: span!(4, 4),
            })),
        },
        _,
    };
}

#[test]
fn test_char_literal() {
    test_lexer! {
        "'a'",
        ast::Token {
            span: span!(0, 3),
            kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
        }
    };

    test_lexer! {
        "'\\u{abcd}'",
        ast::Token {
            span: span!(0, 10),
            kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
        }
    };
}

#[test]
fn test_label() {
    test_lexer! {
        "'asdf 'a' \"foo bar\"",
        ast::Token {
            span: span!(0, 5),
            kind: ast::Kind::Label(ast::LitSource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(5, 6),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(6, 9),
            kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(9, 10),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(10, 19),
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText { source_id: SourceId::EMPTY, escaped: false, wrapped: true })),
        }
    };
}

#[test]
fn test_operators() {
    test_lexer! {
        "+ += - -= * *= / /=",
        ast::Token {
            span: span!(0, 1),
            kind: ast::Kind::Plus,
        },
        ast::Token {
            span: span!(1, 2),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(2, 4),
            kind: ast::Kind::PlusEq,
        },
        ast::Token {
            span: span!(4, 5),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(5, 6),
            kind: ast::Kind::Dash,
        },
        ast::Token {
            span: span!(6, 7),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(7, 9),
            kind: ast::Kind::DashEq,
        },
        ast::Token {
            span: span!(9, 10),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(10, 11),
            kind: ast::Kind::Star,
        },
        ast::Token {
            span: span!(11, 12),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(12, 14),
            kind: ast::Kind::StarEq,
        },
        ast::Token {
            span: span!(14, 15),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(15, 16),
            kind: ast::Kind::Div,
        },
        ast::Token {
            span: span!(16, 17),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(17, 19),
            kind: ast::Kind::SlashEq,
        }
    };
}

#[test]
fn test_idents() {
    test_lexer! {
        "a.checked_div(10)",
        ast::Token {
            span: span!(0, 1),
            kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(1, 2),
            kind: ast::Kind::Dot,
        },
        ast::Token {
            span: span!(2, 13),
            kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(13, 14),
            kind: ast::Kind::Open(ast::Delimiter::Parenthesis),
        },
        ast::Token {
            span: span!(14, 16),
            kind: ast::Kind::Number(ast::NumberSource::Text(ast::NumberText {
                source_id: SourceId::EMPTY,
                is_fractional: false,
                base: ast::NumberBase::Decimal,
                number: span!(14, 16),
                suffix: span!(16, 16),
            })),
        },
        ast::Token {
            span: span!(16, 17),
            kind: ast::Kind::Close(ast::Delimiter::Parenthesis),
        },
    };
}

#[test]
fn test_doc_strings() {
    test_lexer! {
        "//! inner\n/// \"quoted\"",
        ast::Token {
            kind: K![#],
            span: span!(0, 9)
        },
        ast::Token {
            kind: K![!],
            span: span!(0, 9)
        },
        ast::Token {
            kind: K!['['],
            span: span!(0, 9)
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
            span: span!(0, 9)
        },
        ast::Token {
            kind: K![=],
            span: span!(0, 9)
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(3, 9)
        },
        ast::Token {
            kind: K![']'],
            span: span!(0, 9)
        },
        ast::Token {
            kind: ast::Kind::Whitespace,
            span: span!(9, 10)
        },
        ast::Token {
            kind: K![#],
            span: span!(10, 22)
        },
        ast::Token {
            kind: K!['['],
            span: span!(10, 22)
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
            span: span!(10, 22)
        },
        ast::Token {
            kind: K![=],
            span: span!(10, 22)
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(13, 22)
        },
        ast::Token {
            kind: K![']'],
            span: span!(10, 22)
        },
    };
}

#[test]
fn test_multiline_docstring() {
    test_lexer! {
        // /*!
        //  * inner docstr
        //  */
        // /**
        //  * docstr
        //  */
        "/*!\n * inner docstr\n */\n/**\n * docstr\n */",
        ast::Token {
            kind: K![#],
            span: span!(0, 23)
        },
        ast::Token {
            kind: K![!],
            span: span!(0, 23)
        },
        ast::Token {
            kind: K!['['],
            span: span!(0, 23)
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
            span: span!(0, 23)
        },
        ast::Token {
            kind: K![=],
            span: span!(0, 23)
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(3, 21)
        },
        ast::Token {
            kind: K![']'],
            span: span!(0, 23)
        },
        ast::Token {
            kind: ast::Kind::Whitespace,
            span: span!(23, 24)
        },
        ast::Token {
            kind: K![#],
            span: span!(24, 41)
        },
        ast::Token {
            kind: K!['['],
            span: span!(24, 41)
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Doc)),
            span: span!(24, 41)
        },
        ast::Token {
            kind: K![=],
            span: span!(24, 41)
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(27, 39)
        },
        ast::Token {
            kind: K![']'],
            span: span!(24, 41)
        },
    };
}

#[test]
fn test_comment_separators() {
    test_lexer! {
        "///////////////////////////////////\n\
        /*********************************/\n\
        /**********************************\n\
        *                                 *\n\
        ***********************************/",
        ast::Token {
            kind: ast::Kind::Comment,
            span: span!(0, 35)
        },
        ast::Token {
            kind: ast::Kind::Whitespace,
            span: span!(35, 36)
        },
        ast::Token {
            kind: ast::Kind::MultilineComment(true),
            span: span!(36, 71)
        },
        ast::Token {
            kind: ast::Kind::Whitespace,
            span: span!(71, 72)
        },
        ast::Token {
            kind: ast::Kind::MultilineComment(true),
            span: span!(72, 180)
        },
    };
}

#[test]
fn test_template_literals() {
    test_lexer! {
        "`foo ${bar} \\` baz`",
        ast::Token {
            kind: ast::Kind::Open(ast::Delimiter::Empty),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![#],
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['['],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::BuiltIn)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['('],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Literal)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![')'],
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![']'],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Template)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Bang,
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['('],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(1, 5),
        },
        ast::Token {
            kind: ast::Kind::Comma,
            span: span!(5, 7),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
            span: span!(7, 10),
        },
        ast::Token {
            kind: ast::Kind::Comma,
            span: span!(11, 18),
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: true,
                wrapped: false,
            })),
            span: span!(11, 18),
        },
        ast::Token {
            kind: K![')'],
            span: span!(18, 19),
        },
        ast::Token {
            kind: ast::Kind::Close(ast::Delimiter::Empty),
            span: span!(18, 19),
        },
    };
}

#[test]
fn test_template_literals_multi() {
    test_lexer! {
        "`foo ${bar} ${baz}`",
        ast::Token {
            kind: ast::Kind::Open(ast::Delimiter::Empty),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![#],
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['['],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::BuiltIn)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['('],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Literal)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![')'],
            span: span!(0, 1),
        },
        ast::Token {
            kind: K![']'],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::BuiltIn(ast::BuiltIn::Template)),
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Bang,
            span: span!(0, 1),
        },
        ast::Token {
            kind: K!['('],
            span: span!(0, 1),
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(1, 5),
        },
        ast::Token {
            kind: ast::Kind::Comma,
            span: span!(5, 7),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
            span: span!(7, 10),
        },
        ast::Token {
            kind: ast::Kind::Comma,
            span: span!(11, 12),
        },
        ast::Token {
            kind: ast::Kind::Str(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: false,
            })),
            span: span!(11, 12),
        },
        ast::Token {
            kind: ast::Kind::Comma,
            span: span!(12, 14),
        },
        ast::Token {
            kind: ast::Kind::Ident(ast::LitSource::Text(SourceId::EMPTY)),
            span: span!(14, 17),
        },
        ast::Token {
            kind: K![')'],
            span: span!(18, 19),
        },
        ast::Token {
            kind: ast::Kind::Close(ast::Delimiter::Empty),
            span: span!(18, 19),
        },
    };
}

#[test]
fn test_literals() {
    test_lexer! {
        r#"b"""#,
        ast::Token {
            span: span!(0, 3),
            kind: ast::Kind::ByteStr(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: true,
            })),
        },
    };

    test_lexer! {
        r#"b"hello world""#,
        ast::Token {
            span: span!(0, 14),
            kind: ast::Kind::ByteStr(ast::StrSource::Text(ast::StrText {
                source_id: SourceId::EMPTY,
                escaped: false,
                wrapped: true,
            })),
        },
    };

    test_lexer! {
        "b'\\\\''",
        ast::Token {
            span: span!(0, 6),
            kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
        },
    };

    test_lexer! {
        "'label 'a' b'a'",
        ast::Token {
            span: span!(0, 6),
            kind: ast::Kind::Label(ast::LitSource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(6, 7),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(7, 10),
            kind: ast::Kind::Char(ast::CopySource::Text(SourceId::EMPTY)),
        },
        ast::Token {
            span: span!(10, 11),
            kind: ast::Kind::Whitespace,
        },
        ast::Token {
            span: span!(11, 15),
            kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
        },
    };

    test_lexer! {
        "b'a'",
        ast::Token {
            span: span!(0, 4),
            kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
        },
    };

    test_lexer! {
        "b'\\n'",
        ast::Token {
            span: span!(0, 5),
            kind: ast::Kind::Byte(ast::CopySource::Text(SourceId::EMPTY)),
        },
    };
}

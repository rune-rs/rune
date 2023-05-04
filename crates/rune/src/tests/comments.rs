prelude!();

use CompileErrorKind::*;
use ParseErrorKind::*;

#[test]
fn test_non_terminated_multiline_comments() {
    assert_errors! {
        r#"/* foo"#,
        span, ParseError(ExpectedMultilineCommentTerm) => {
            assert_eq!(span, span!(0, 6));
        }
    };

    assert_errors! {
        r#"/*
        foo
        bar"#,
        span, ParseError(ExpectedMultilineCommentTerm) => {
            assert_eq!(span, span!(0, 26));
        }
    };

    assert_errors! {
        r#"
        foo
        /*
        foo
        bar"#,
        span, ParseError(ExpectedMultilineCommentTerm) => {
            assert_eq!(span, span!(21, 47));
        }
    };
}

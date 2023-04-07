prelude!();

use ParseErrorKind::*;

#[test]
fn test_non_terminated_multiline_comments() {
    assert_parse_error! {
        r#"/* foo"#,
        span, ExpectedMultilineCommentTerm => {
            assert_eq!(span, span!(0, 6));
        }
    };

    assert_parse_error! {
        r#"/*
        foo
        bar"#,
        span, ExpectedMultilineCommentTerm => {
            assert_eq!(span, span!(0, 26));
        }
    };

    assert_parse_error! {
        r#"
        foo
        /*
        foo
        bar"#,
        span, ExpectedMultilineCommentTerm => {
            assert_eq!(span, span!(21, 47));
        }
    };
}

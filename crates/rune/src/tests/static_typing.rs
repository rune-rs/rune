prelude!();

use ErrorKind::*;

#[test]
fn deny_static_typing_function() {
    assert_errors! {
        "fn foo() -> Bar {}",
        span!(0, 18), Custom { error } => {
            assert_eq!(error.to_string(), "Adding a return type in functions is not supported");
        }
    }
}

#[test]
fn deny_static_typing_field() {
    assert_errors! {
        "struct Struct { foo: Bar }",
        span!(16, 24), Custom { error } => {
            assert_eq!(error.to_string(), "Static typing on fields is not supported");
        }
    }
}

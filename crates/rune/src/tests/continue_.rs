prelude!();

use ErrorKind::*;

#[test]
fn test_continue_not_in_loop() {
    assert_errors! {
        r#"pub fn main() { continue }"#,
        span!(16, 24), ContinueUnsupported,
    };
}

#[test]
fn test_continue_missing_label() {
    assert_errors! {
        r#"pub fn main() { 'existing: loop { loop { continue 'missing; } } }"#,
        span!(41, 58), MissingLabel { label } => {
            assert_eq!(&*label, "missing");
        }
    };
}

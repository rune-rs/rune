prelude!();

use CompileErrorKind::*;

#[test]
fn test_bad_attributes() {
    assert_errors! {
        r#"pub fn main() { #[foo] #[bar] let x = 1; }"#,
        span!(16, 29), Custom { message } => {
            assert_eq!(message.as_ref(), "attributes are not supported");
        }
    };
}

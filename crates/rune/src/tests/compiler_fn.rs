prelude!();

use CompileErrorKind::*;

#[test]
fn test_fn_const_async() {
    assert_compile_error! {
        r#"pub const async fn main() {}"#,
        span, FnConstAsyncConflict => {
            assert_eq!(span, span!(4, 15));
        }
    };

    assert_compile_error! {
        r#"pub const fn main() { yield true }"#,
        span, FnConstNotGenerator => {
            assert_eq!(span, span!(0, 34));
        }
    };
}

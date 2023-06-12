prelude!();

use ErrorKind::*;

#[test]
fn test_fn_const_async() {
    assert_errors! {
        r#"pub const async fn main() {}"#,
        span!(4, 15), FnConstAsyncConflict
    };

    assert_errors! {
        r#"pub const fn main() { yield true }"#,
        span!(22, 32), YieldInConst
    };
}

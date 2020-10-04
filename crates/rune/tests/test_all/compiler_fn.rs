use rune::testing::*;

#[test]
fn test_fn_const_async() {
    assert_compile_error! {
        r#"const async fn main() {}"#,
        span, FnConstAsyncConflict => {
            assert_eq!(span, Span::new(0, 11));
        }
    };

    assert_compile_error! {
        r#"const fn main() { yield true }"#,
        span, FnConstNotGenerator => {
            assert_eq!(span, Span::new(0, 30));
        }
    };
}

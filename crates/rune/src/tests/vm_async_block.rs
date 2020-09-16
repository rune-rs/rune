#[test]
fn test_async_block() {
    assert_eq! {
        21,
        rune! {
            i64 => r#"
            async fn foo(value) {
                let output = value.await;
                output
            }

            async fn main() {
                let value = 42;
                foo(async { value }).await / foo(async { 2 }).await
            }
            "#
        }
    };
}

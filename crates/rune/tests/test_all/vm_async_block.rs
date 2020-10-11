#[test]
fn test_async_block() {
    assert_eq! {
        21,
        rune! { i64 =>
            async fn foo(value) {
                let output = value.await;
                output
            }

            pub async fn main() {
                let value = 42;
                foo(async { value }).await / foo(async { 2 }).await
            }
        }
    };
}

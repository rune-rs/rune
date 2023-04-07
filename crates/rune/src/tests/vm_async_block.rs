prelude!();

#[test]
fn test_async_block() {
    let out: i64 = rune! {
        async fn foo(value) {
            let output = value.await;
            output
        }

        pub async fn main() {
            let value = 42;
            foo(async { value }).await / foo(async { 2 }).await
        }
    };
    assert_eq!(out, 21);
}

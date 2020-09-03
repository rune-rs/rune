use rune_testing::*;

fn main() -> runestick::Result<()> {
    let fn_ptr: FnPtr = rune! {
        FnPtr => r#"
        fn foo(a, b) {
            a + b
        }

        fn main() {
            foo
        }
        "#
    };

    println!("{}", fn_ptr.call::<(i64, i64), i64>((1, 3))?);
    println!("{}", fn_ptr.call::<(i64, i64), i64>((2, 6))?);
    Ok(())
}

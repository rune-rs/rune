use rune_testing::*;

fn main() -> runestick::Result<()> {
    let function: Function = rune! {
        Function => r#"
        fn foo(a, b) {
            a + b
        }

        fn main() {
            foo
        }
        "#
    };

    println!("{}", function.call::<(i64, i64), i64>((1, 3))?);
    println!("{}", function.call::<(i64, i64), i64>((2, 6))?);
    Ok(())
}

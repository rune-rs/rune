use runestick::Function;

fn main() -> runestick::Result<()> {
    let function: Function = rune_tests::rune! { Function =>
        fn foo(a, b) {
            a + b
        }

        pub fn main() {
            foo
        }
    };

    println!("{}", function.call::<(i64, i64), i64>((1, 3))?);
    println!("{}", function.call::<(i64, i64), i64>((2, 6))?);
    Ok(())
}

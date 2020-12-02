use rune::testing::run;
use runestick::VecTuple;
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let input: VecTuple<(i64, String)> = VecTuple::new((1, String::from("Hello")));

    let output: VecTuple<(i64, String)> = run(
        &context,
        r#"
        pub fn calc(input) {
            let a = input[0] + 1;
            let b = `{input[1]} World`;
            [a, b]
        }
        "#,
        &["calc"],
        (input,),
    )?;

    let VecTuple((a, b)) = output;
    println!("({:?}, {:?})", a, b);
    Ok(())
}

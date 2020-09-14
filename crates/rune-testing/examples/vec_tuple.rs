use rune_testing::run;
use runestick::VecTuple;

fn main() -> runestick::Result<()> {
    let input: VecTuple<(i64, String)> = VecTuple::new((1, String::from("Hello")));

    let output: VecTuple<(i64, String)> = run(
        &["calc"],
        (input,),
        r#"
        fn calc(input) {
            let a = input[0] + 1;
            let b = `{input[1]} World`;
            [a, b]
        }
        "#,
    )?;

    let VecTuple((a, b)) = output;
    println!("({:?}, {:?})", a, b);
    Ok(())
}

use rune::testing::*;

fn main() -> runestick::Result<()> {
    let input: Vec<i64> = vec![1, 2, 3, 4];

    let output: Vec<i64> = run(
        &["calc"],
        (input,),
        r#"
        fn calc(input) {
            let output = 0;

            for value in input {
                output += value;
            }

            [output]
        }
        "#,
    )?;

    println!("{:?}", output);
    Ok(())
}

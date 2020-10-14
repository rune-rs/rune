use rune::testing::*;
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let input: Vec<i64> = vec![1, 2, 3, 4];

    let output: Vec<i64> = run(
        &context,
        &["calc"],
        (input,),
        r#"
        pub fn calc(input) {
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

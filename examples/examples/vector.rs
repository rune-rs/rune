use rune_tests::*;
use std::sync::Arc;

fn main() -> rune::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let input: Vec<i64> = vec![1, 2, 3, 4];

    let output: Vec<i64> = run(
        &context,
        r#"
        pub fn calc(input) {
            let output = 0;

            for value in input {
                output += value;
            }

            [output]
        }
        "#,
        &["calc"],
        (input,),
    )?;

    println!("{:?}", output);
    Ok(())
}

use rune_tests::run;
use std::sync::Arc;

fn main() -> runestick::Result<()> {
    let context = Arc::new(rune_modules::default_context()?);

    let object: (i64, i64) = run(
        &context,
        r#"
        pub fn calc(input) {
            (input.0 + 1, input.1 + 2)
        }
        "#,
        &["calc"],
        ((1, 2),),
    )?;

    println!("{:?}", object);
    Ok(())
}

use rune::testing::run;

fn main() -> runestick::Result<()> {
    let object: (i64, i64) = run(
        &["calc"],
        ((1, 2),),
        r#"
        fn calc(input) {
            (input.0 + 1, input.1 + 2)
        }
        "#,
    )?;

    println!("{:?}", object);
    Ok(())
}

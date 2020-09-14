use rune_testing::*;

#[test]
fn test_external_function() -> runestick::Result<()> {
    // NB: here we test passing the function from one virtual machine instance
    // into another, making sure that the function holds everything it needs to
    // be called.

    let function: Function = run(
        &["main"],
        (),
        r#"
        fn test() { 42 }
        fn main() { test }
        "#,
    )?;

    let output: i64 = run(
        &["main"],
        (function,),
        r#"
        fn main(f) { f() }
        "#,
    )?;

    assert_eq!(42, output);
    Ok(())
}

#[test]
fn test_external_generator() -> runestick::Result<()> {
    // NB: here we test passing the generator from one virtual machine instance
    // into another, making sure that the function holds everything it needs to
    // be called.

    let function: Function = run(
        &["main"],
        (),
        r#"
        fn test() { yield 42; }
        fn main() { test }
        "#,
    )?;

    let output: (Option<i64>, Option<i64>) = run(
        &["main"],
        (function,),
        r#"
        fn main(f) { let gen = f(); (gen.next(), gen.next()) }
        "#,
    )?;

    assert_eq!((Some(42), None), output);
    Ok(())
}

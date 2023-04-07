use rune_tests::prelude::*;

#[test]
fn test_external_function() -> Result<()> {
    let context = Context::with_default_modules()?;

    // NB: here we test passing the function from one virtual machine instance
    // into another, making sure that the function holds everything it needs to
    // be called.

    let function: Function = run(
        &context,
        r#"
        fn test() { 42 }
        pub fn main() { test }
        "#,
        ["main"],
        (),
    )?;

    let output: i64 = run(
        &context,
        r#"
        pub fn main(f) { f() }
        "#,
        ["main"],
        (function,),
    )?;

    assert_eq!(42, output);
    Ok(())
}

#[test]
fn test_external_generator() -> Result<()> {
    let context = Context::with_default_modules()?;

    // NB: here we test passing the generator from one virtual machine instance
    // into another, making sure that the function holds everything it needs to
    // be called.

    let function: Function = run(
        &context,
        r#"
        fn test() { yield 42; }
        pub fn main() { test }
        "#,
        ["main"],
        (),
    )?;

    let output: (Option<i64>, Option<i64>) = run(
        &context,
        r#"
        pub fn main(f) { let gen = f(); (gen.next(), gen.next()) }
        "#,
        ["main"],
        (function,),
    )?;

    assert_eq!((Some(42), None), output);
    Ok(())
}

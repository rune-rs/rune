use rune_testing::*;

#[test]
fn test_external_fn_ptr() -> Result<()> {
    let fn_ptr: FnPtr = run(
        &["main"],
        (),
        r#"
        fn test() { 42 }
        fn main() { test }
        "#,
    )?;

    println!("here");

    let output: i64 = run(
        &["main"],
        (fn_ptr,),
        r#"
        fn main(f) { f() }
        "#,
    )?;

    println!("here");
    assert_eq!(42, output);
    Ok(())
}

#[test]
fn test_external_generator() -> Result<()> {
    let fn_ptr: FnPtr = run(
        &["main"],
        (),
        r#"
        fn test() { yield 42; }
        fn main() { test }
        "#,
    )?;

    let output: (Option<i64>, Option<i64>) = run(
        &["main"],
        (fn_ptr,),
        r#"
        fn main(f) { let gen = f(); (gen.next().await, gen.next().await) }
        "#,
    )?;

    // TODO: reverse function argument order!
    assert_eq!(Some(42), output.1);
    assert_eq!(None, output.0);
    Ok(())
}

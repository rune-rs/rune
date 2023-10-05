prelude!();

/// Here we're constructing a unit-specific function pointer and ensuring that
/// the vm execution can handle it correctly.
#[test]
fn vm_execution_unit_fn() -> Result<()> {
    let context = Context::with_default_modules()?;

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

/// Here we're constructing multiple different kinds of function pointers and
/// ensuring that the vm execution can handle them correctly.
#[test]
fn vm_execution_with_complex_external() -> Result<()> {
    let mut m = Module::new();
    m.function("external", || 42i64).build()?;

    let mut c1 = Context::with_default_modules()?;
    c1.install(m)?;

    let c2 = Context::with_default_modules()?;

    let function: Function = run(
        &c1,
        r#"
        fn unit() { 84 }
        fn function() { (external, unit) }
        pub fn main() { function }
        "#,
        ["main"],
        (),
    )?;

    let (o1, o2): (i64, i64) = run(
        &c2,
        r#"
        pub fn main(f) {
            let (f1, f2) = f();
            (f1(), f2())
        }
        "#,
        ["main"],
        (function,),
    )?;

    assert_eq!(o1, 42);
    assert_eq!(o2, 84);
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

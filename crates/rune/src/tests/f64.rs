prelude!();

#[test]
fn test_f64_ceil() -> Result<()> {
    let context = Context::with_default_modules()?;
    let value: f64 = run(&context, "(1.0 + f64::EPSILON).ceil()", (), true)?;
    assert_eq!(value, 2.0);

    Ok(())
}

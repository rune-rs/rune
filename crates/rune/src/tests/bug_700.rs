prelude!();

/// See https://github.com/rune-rs/rune/issues/700
#[test]
pub fn test_bug_700() -> Result<()> {
    let context = Context::new();

    let mut sources = sources! {
        entry => {
            pub fn main(argument) {
                || argument
            }
        }
    };

    let unit = prepare(&mut sources).with_context(&context).build()?;
    let mut vm = Vm::new(Arc::new(context.runtime()?), Arc::new(unit));

    let value = vm.call(["main"], (42,))?;
    let function = from_value::<Function>(value)?;

    let output: i64 = function.call(()).into_result()?;
    assert_eq!(output, 42);

    // This should error, because the function is missing the environment variable.
    let error = vm.call(function.type_hash(), ()).unwrap_err();

    assert_eq!(
        error.into_kind(),
        VmErrorKind::BadArgumentCount {
            actual: 0,
            expected: 1
        }
    );

    // We call with an argument, but it's not a tuple, which is required for the environment.
    let error = vm.call(function.type_hash(), (0,)).unwrap_err();

    assert_eq!(
        error.into_kind(),
        VmErrorKind::Expected {
            expected: TypeInfo::any::<OwnedTuple>(),
            actual: TypeInfo::named::<i64>()
        }
    );

    let value = vm.call(function.type_hash(), ((84,),)).unwrap();
    let output: i64 = from_value::<i64>(value)?;
    assert_eq!(output, 84);
    Ok(())
}

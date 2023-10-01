prelude!();

use std::cmp::Ordering;
use std::sync::Arc;

#[test]
fn assign_ops_struct() -> Result<()> {
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $derived:tt, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External {
                value: i64,
                field: i64,
                #[rune($derived)]
                derived: i64,
                #[rune($derived = External::custom)]
                custom: i64,
            }

            impl External {
                fn value(&mut self, value: i64) {
                    self.value $($op)* value;
                }

                fn field(&mut self, value: i64) {
                    self.field $($op)* value;
                }

                fn custom(&mut self, value: i64) {
                    self.custom $($op)* value;
                }
            }

            let mut module = Module::new();
            module.ty::<External>()?;

            module.associated_function(Protocol::$protocol, External::value)?;
            module.field_function(Protocol::$protocol, "field", External::field)?;

            let mut context = Context::with_default_modules()?;
            context.install(module)?;

            let mut sources = Sources::new();
            sources.insert(Source::new(
                "test",
                format!(r#"
                pub fn type(number) {{
                    number {op} {arg};
                    number.field {op} {arg};
                    number.derived {op} {arg};
                    number.custom {op} {arg};
                }}
                "#, op = stringify!($($op)*), arg = stringify!($arg)),
            )?)?;

            let unit = prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()?), unit);

            {
                let mut foo = External::default();
                foo.value = $initial;
                foo.field = $initial;
                foo.derived = $initial;
                foo.custom = $initial;

                let output = vm.try_clone()?.call(["type"], (&mut foo,))?;

                assert_eq!(foo.value, $expected, "{} != {} (value)", foo.value, $expected);
                assert_eq!(foo.field, $expected, "{} != {} (field)", foo.field, $expected);
                assert_eq!(foo.derived, $expected, "{} != {} (derived)", foo.derived, $expected);
                assert_eq!(foo.custom, $expected, "{} != {} (custom)", foo.custom, $expected);
                assert!(matches!(output, Value::EmptyTuple));
            }
        }};
    }

    test_case!([+=], ADD_ASSIGN, add_assign, 0, 3, 3);
    test_case!([-=], SUB_ASSIGN, sub_assign, 4, 3, 1);
    test_case!([*=], MUL_ASSIGN, mul_assign, 8, 2, 16);
    test_case!([/=], DIV_ASSIGN, div_assign, 8, 3, 2);
    test_case!([&=], BIT_AND_ASSIGN, bit_and_assign, 0b1001, 0b0011, 0b0001);
    test_case!([|=], BIT_OR_ASSIGN, bit_or_assign, 0b1001, 0b0011, 0b1011);
    test_case!([^=], BIT_XOR_ASSIGN, bit_xor_assign, 0b1001, 0b0011, 0b1010);
    test_case!([<<=], SHL_ASSIGN, shl_assign, 0b1001, 0b0001, 0b10010);
    test_case!([>>=], SHR_ASSIGN, shr_assign, 0b1001, 0b0001, 0b100);
    test_case!([%=], REM_ASSIGN, rem_assign, 25, 10, 5);
    Ok(())
}

#[test]
fn assign_ops_tuple() -> Result<()> {
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $derived:tt, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External(i64, i64, #[rune($derived)] i64, #[rune($derived = External::custom)] i64);

            impl External {
                fn value(&mut self, value: i64) {
                    self.0 $($op)* value;
                }

                fn field(&mut self, value: i64) {
                    self.1 $($op)* value;
                }

                fn custom(&mut self, value: i64) {
                    self.3 $($op)* value;
                }
            }

            let mut module = Module::new();
            module.ty::<External>()?;

            module.associated_function(Protocol::$protocol, External::value)?;
            module.index_function(Protocol::$protocol, 1, External::field)?;

            let mut context = Context::with_default_modules()?;
            context.install(module)?;

            let mut sources = Sources::new();
            sources.insert(Source::new(
                "test",
                format!(r#"
                pub fn type(number) {{
                    number {op} {arg};
                    number.1 {op} {arg};
                    number.2 {op} {arg};
                    number.3 {op} {arg};
                }}
                "#, op = stringify!($($op)*), arg = stringify!($arg)),
            )?)?;

            let unit = prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()?), unit);

            {
                let mut foo = External::default();
                foo.0 = $initial;
                foo.1 = $initial;
                foo.2 = $initial;
                foo.3 = $initial;

                let output = vm.try_clone()?.call(["type"], (&mut foo,))?;

                assert_eq!(foo.0, $expected, "{} != {} (value .0)", foo.0, $expected);
                assert_eq!(foo.1, $expected, "{} != {} (field .1)", foo.1, $expected);
                assert_eq!(foo.2, $expected, "{} != {} (derived .2)", foo.2, $expected);
                assert_eq!(foo.3, $expected, "{} != {} (custom .3)", foo.3, $expected);
                assert!(matches!(output, Value::EmptyTuple));
            }
        }};
    }

    test_case!([+=], ADD_ASSIGN, add_assign, 0, 3, 3);
    test_case!([-=], SUB_ASSIGN, sub_assign, 4, 3, 1);
    test_case!([*=], MUL_ASSIGN, mul_assign, 8, 2, 16);
    test_case!([/=], DIV_ASSIGN, div_assign, 8, 3, 2);
    test_case!([&=], BIT_AND_ASSIGN, bit_and_assign, 0b1001, 0b0011, 0b0001);
    test_case!([|=], BIT_OR_ASSIGN, bit_or_assign, 0b1001, 0b0011, 0b1011);
    test_case!([^=], BIT_XOR_ASSIGN, bit_xor_assign, 0b1001, 0b0011, 0b1010);
    test_case!([<<=], SHL_ASSIGN, shl_assign, 0b1001, 0b0001, 0b10010);
    test_case!([>>=], SHR_ASSIGN, shr_assign, 0b1001, 0b0001, 0b100);
    test_case!([%=], REM_ASSIGN, rem_assign, 25, 10, 5);
    Ok(())
}

#[test]
fn ordering_struct() -> Result<()> {
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External {
                value: i64,
            }

            impl External {
                fn value(&self, value: i64) -> Option<Ordering> {
                    PartialOrd::partial_cmp(&self.value, &value)
                }
            }

            let mut module = Module::new();
            module.ty::<External>()?;

            module.associated_function(Protocol::$protocol, External::value)?;

            let mut context = Context::with_default_modules()?;
            context.install(module)?;

            let mut sources = Sources::new();
            sources.insert(Source::new(
                "test",
                format!(r#"
                pub fn type(number) {{
                    number {op} {arg}
                }}
                "#, op = stringify!($($op)*), arg = stringify!($arg)),
            )?)?;

            let unit = prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()?), unit);

            {
                let mut foo = External::default();
                foo.value = $initial;

                let output = vm.try_clone()?.call(["type"], (&mut foo,))?;
                let a = <bool as FromValue>::from_value(output).into_result()?;

                assert_eq!(a, $expected, "{} != {} (value)", foo.value, $expected);
            }
        }};
    }

    test_case!([<], PARTIAL_CMP, 1, 2, true);
    test_case!([<], PARTIAL_CMP, 2, 1, false);

    test_case!([>], PARTIAL_CMP, 2, 1, true);
    test_case!([>], PARTIAL_CMP, 1, 2, false);

    test_case!([>=], PARTIAL_CMP, 3, 2, true);
    test_case!([>=], PARTIAL_CMP, 2, 2, true);
    test_case!([>=], PARTIAL_CMP, 1, 2, false);

    test_case!([<=], PARTIAL_CMP, 2, 3, true);
    test_case!([<=], PARTIAL_CMP, 2, 2, true);
    test_case!([<=], PARTIAL_CMP, 2, 1, false);
    Ok(())
}

#[test]
fn eq_struct() -> Result<()> {
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External {
                value: i64,
            }

            impl External {
                fn value(&self, value: i64) -> bool {
                    self.value $($op)* value
                }
            }

            let mut module = Module::new();
            module.ty::<External>()?;

            module.associated_function(Protocol::$protocol, External::value)?;

            let mut context = Context::with_default_modules()?;
            context.install(module)?;

            let mut sources = Sources::new();
            sources.insert(Source::new(
                "test",
                format!(r#"
                pub fn type(number) {{
                    number {op} {arg}
                }}
                "#, op = stringify!($($op)*), arg = stringify!($arg)),
            )?)?;

            let unit = prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()?), unit);

            {
                let mut foo = External::default();
                foo.value = $initial;

                let output = vm.try_clone()?.call(["type"], (&mut foo,))?;
                let a = <bool as FromValue>::from_value(output).into_result()?;

                assert_eq!(a, $expected, "{} != {} (value)", foo.value, $expected);
            }
        }};
    }

    test_case!([==], PARTIAL_EQ, 2, 2, true);
    test_case!([==], PARTIAL_EQ, 2, 1, false);
    Ok(())
}

use rune::runtime::Protocol;
use rune::{Any, Context, Module, Source, Sources, Value, Vm};
use rune_tests::*;
use std::sync::Arc;

#[test]
fn test_external_ops_struct() -> rune::Result<()> {
    /// Test case for a single operation.
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $derived:tt, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External {
                value: i64,
                field: i64,
                #[rune($derived)]
                derived: i64,
                #[rune($derived = "External::custom")]
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

            module.inst_fn(Protocol::$protocol, External::value)?;
            module.field_fn(Protocol::$protocol, "field", External::field)?;

            let mut context = Context::with_default_modules()?;
            context.install(&module)?;

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
            ));

            let unit = rune::prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()), unit);

            {
                let mut foo = External::default();
                foo.value = $initial;
                foo.field = $initial;
                foo.derived = $initial;
                foo.custom = $initial;

                let output = vm.clone().call(["type"], (&mut foo,))?;

                assert_eq!(foo.value, $expected, "{} != {} (value)", foo.value, $expected);
                assert_eq!(foo.field, $expected, "{} != {} (field)", foo.value, $expected);
                assert_eq!(foo.derived, $expected, "{} != {} (derived)", foo.value, $expected);
                assert_eq!(foo.custom, $expected, "{} != {} (custom)", foo.value, $expected);
                assert!(matches!(output, Value::Unit));
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
#[ignore = "Currently does not work, but should!"]
fn test_external_ops_tuple() -> rune::Result<()> {
    /// Test case for a single operation.
    macro_rules! test_case {
        ([$($op:tt)*], $protocol:ident, $derived:tt, $initial:literal, $arg:literal, $expected:literal) => {{
            #[derive(Debug, Default, Any)]
            struct External(i64, i64, #[rune($derived)] i64, #[rune($derived = "External::custom")] i64);

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

            module.inst_fn(Protocol::$protocol, External::value)?;
            module.index_fn(Protocol::$protocol, 1, External::field)?;

            let mut context = Context::with_default_modules()?;
            context.install(&module)?;

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
            ));

            let unit = rune::prepare(&mut sources)
                .with_context(&context)
                .build()?;

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()), unit);

            {
                let mut foo = External::default();
                foo.0 = $initial;
                foo.1 = $initial;
                foo.2 = $initial;
                foo.3 = $initial;

                let output = vm.clone().call(["type"], (&mut foo,))?;

                assert_eq!(foo.0, $expected, "{} != {} (value)", foo.0, $expected);
                assert_eq!(foo.1, $expected, "{} != {} (field)", foo.0, $expected);
                assert_eq!(foo.2, $expected, "{} != {} (derived)", foo.0, $expected);
                assert_eq!(foo.3, $expected, "{} != {} (custom)", foo.0, $expected);
                assert!(matches!(output, Value::Unit));
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

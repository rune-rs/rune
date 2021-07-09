use rune::{Diagnostics, Options, Sources};
use runestick::{Any, Context, Module, Protocol, Source, Value, Vm};
use std::sync::Arc;

#[test]
fn test_external_ops() {
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
            module.ty::<External>().unwrap();

            module
                .inst_fn(Protocol::$protocol, External::value)
                .unwrap();

            module
                .field_fn(Protocol::$protocol, "field", External::field)
                .unwrap();

            let mut context = Context::with_default_modules().unwrap();
            context.install(&module).unwrap();

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

            let mut diagnostics = Diagnostics::new();

            let unit = rune::load_sources(
                &context,
                &Options::default(),
                &mut sources,
                &mut diagnostics,
            )
            .unwrap();

            let unit = Arc::new(unit);

            let vm = Vm::new(Arc::new(context.runtime()), unit);

            {
                let mut test = External::default();
                test.value = $initial;
                test.field = $initial;
                test.derived = $initial;
                test.custom = $initial;

                let output = vm.clone().call(&["type"], (&mut test,)).unwrap();

                assert_eq!(test.value, $expected, "{} != {} (value)", test.value, $expected);
                assert_eq!(test.field, $expected, "{} != {} (field)", test.value, $expected);
                assert_eq!(test.derived, $expected, "{} != {} (derived)", test.value, $expected);
                assert_eq!(test.custom, $expected, "{} != {} (custom)", test.value, $expected);
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
}

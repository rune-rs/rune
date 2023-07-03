//! Tests for derive(Any) generates the necessary metadata to match over fields.

prelude!();

#[test]
fn struct_match() {
    #[derive(Any, Clone, Copy)]
    struct Struct {
        #[rune(get)]
        a: u32,
        #[rune(get)]
        b: u32,
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Struct>()?;
        Ok(module)
    }

    let m = make_module().expect("failed make module");

    let e = Struct { a: 40, b: 41 };

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { Struct { .. } => 2, _ => 0 } }
        },
        2
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { Struct { a, .. } => a, _ => 0 } }
        },
        40
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { Struct { b, .. } => b, _ => 0 } }
        },
        41
    );

    assert_eq!(
        rune_n! {
            &m,
            (e,),
            i64 => pub fn main(v) { match v { Struct { a, b } => a + b, _ => 0 } }
        },
        81
    );
}

#[test]
fn enum_match() {
    #[derive(Any, Clone, Copy)]
    enum Enum {
        Success,
        Failed,
        Aborted,
        Errored,
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Enum>()?;
        Ok(module)
    }

    let m = make_module().expect("failed make module");

    macro_rules! test {
        ($expected:ident, $other:ident) => {{
            let e = Enum::$expected;

            assert_eq!(
                rune_n! {
                    &m,
                    (e,),
                    i64 => pub fn main(v) { match v { Enum::$expected => 1, Enum::$other => 2, _ => 0 } }
                },
                1
            );

            // TODO: Eventually we want this to be fine - we want the `{ .. }`
            // pattern to match *any* kind of enum.
            // assert_eq!(
            //     rune_n! {
            //         &m,
            //         (e,),
            //         i64 => pub fn main(v) { match v { Enum::$expected { .. } => 1, Enum::$other => 2, _ => 0 } }
            //     },
            //     1
            // );

            let e = Enum::$other;

            assert_eq!(
                rune_n! {
                    &m,
                    (e,),
                    i64 => pub fn main(v) { match v { Enum::$expected => 1, Enum::$other => 2, _ => 0 } }
                },
                2
            );
        }}
    }

    // Do a trip around the enum.
    test!(Success, Failed);
    test!(Failed, Aborted);
    test!(Aborted, Errored);
    test!(Errored, Success);
}

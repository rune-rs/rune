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

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Struct { .. } => 2, _ => 0 } }
    };

    assert_eq!(n, 2);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Struct { a, .. } => a, _ => 0 } }
    };

    assert_eq!(n, 40);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Struct { b, .. } => b, _ => 0 } }
    };

    assert_eq!(n, 41);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Struct { a, b } => a + b, _ => 0 } }
    };

    assert_eq!(n, 81);
}

#[test]
fn simple_enum_match() {
    #[derive(Debug, Any, Clone, Copy)]
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

            let n: u32 = rune_n! {
                mod m,
                (e,),
                pub fn main(v) { match v { Enum::$expected => 1, Enum::$other => 2, _ => 0 } }
            };

            assert_eq!(n, 1);

            let n: u32 = rune_n! {
                mod m,
                (e,),
                pub fn main(v) { match v { Enum::$expected { .. } => 1, Enum::$other => 2, _ => 0 } }
            };

            assert_eq!(n, 1);

            let e = Enum::$other;

            let n: u32 = rune_n! {
                mod m,
                (e,),
                pub fn main(v) { match v { Enum::$expected => 1, Enum::$other => 2, _ => 0 } }
            };

            assert_eq!(n, 2);
        }}
    }

    // Do a trip around the enum.
    test!(Success, Failed);
    test!(Failed, Aborted);
    test!(Aborted, Errored);
    test!(Errored, Success);
}
#[test]
fn field_enum_match() {
    #[derive(Debug, Any, Clone, Copy)]
    enum Enum {
        Unnamed(#[rune(get)] u32, #[rune(get)] u32),
        Named {
            #[rune(get)]
            a: u32,
            #[rune(get)]
            b: u32,
        },
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::new();
        module.ty::<Enum>()?;
        Ok(module)
    }

    let m = make_module().expect("failed make module");

    let e = Enum::Unnamed(1, 2);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Enum::Unnamed(a, ..) => a, _ => 0 } }
    };

    assert_eq!(n, 1);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Enum::Unnamed(_, b, ..) => b, _ => 0 } }
    };

    assert_eq!(n, 2);

    let e = Enum::Named { a: 1, b: 2 };

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Enum::Named { a, .. } => a, _ => 0 } }
    };

    assert_eq!(n, 1);

    let n: u32 = rune_n! {
        mod m,
        (e,),
        pub fn main(v) { match v { Enum::Named { b, .. } => b, _ => 0 } }
    };

    assert_eq!(n, 2);
}

macro_rules! test_op {
    ($ty:ty => $lhs:literal $op:tt $rhs:literal = $result:literal) => {{
        let program = format!(
            r#"const A = {lhs}; const B = {rhs}; const VALUE = A {op} B; fn main() {{ VALUE }}"#,
            lhs = $lhs, rhs = $rhs, op = stringify!($op),
        );

        assert_eq!(
            $result,
            rune_s!($ty => &program),
            concat!("expected ", stringify!($result), " out of program `{}`"),
            program
        );
    }}
}

#[test]
fn test_const_values() {
    assert_eq!(true, rune!(bool => const VALUE = true; fn main() { VALUE }));

    assert_eq!(
        "Hello World",
        rune!(String => const VALUE = "Hello World"; fn main() { VALUE })
    );

    assert_eq!(
        "Hello World 1 1.0 true",
        rune_s!(String => r#"
            const VALUE = `Hello {WORLD} {A} {B} {C}`;
            const WORLD = "World";
            const A = 1;
            const B = 1.0;
            const C = true;
            fn main() { VALUE }
        "#)
    );
}

#[test]
fn test_integer_ops() {
    test_op!(i64 => 1 + 2 = 3);
    test_op!(i64 => 2 - 1 = 1);
    test_op!(i64 => 8 / 2 = 4);
    test_op!(i64 => 8 * 2 = 16);
    test_op!(i64 => 0b1010 << 2 = 0b101000);
    test_op!(i64 => 0b1010 >> 2 = 0b10);
    test_op!(bool => 1 < 2 = true);
    test_op!(bool => 2 < 2 = false);
    test_op!(bool => 1 <= 1 = true);
    test_op!(bool => 2 <= 1 = false);
    test_op!(bool => 3 > 2 = true);
    test_op!(bool => 2 > 2 = false);
    test_op!(bool => 1 >= 1 = true);
    test_op!(bool => 0 >= 2 = false);
}

macro_rules! test_float_op {
    ($ty:ty => $lhs:literal $op:tt $rhs:literal = $result:literal) => {{
        let program = format!(
            r#"const A = {lhs}.0; const B = {rhs}.0; const VALUE = A {op} B; fn main() {{ VALUE }}"#,
            lhs = $lhs, rhs = $rhs, op = stringify!($op),
        );

        assert_eq!(
            $result,
            rune_s!($ty => &program),
            concat!("expected ", stringify!($result), " out of program `{}`"),
            program
        );
    }}
}

#[test]
fn test_float_ops() {
    test_float_op!(f64 => 1 + 2 = 3f64);
    test_float_op!(f64 => 2 - 1 = 1f64);
    test_float_op!(f64 => 8 / 2 = 4f64);
    test_float_op!(f64 => 8 * 2 = 16f64);
    test_float_op!(bool => 1 < 2 = true);
    test_float_op!(bool => 2 < 2 = false);
    test_float_op!(bool => 1 <= 1 = true);
    test_float_op!(bool => 2 <= 1 = false);
    test_float_op!(bool => 3 > 2 = true);
    test_float_op!(bool => 2 > 2 = false);
    test_float_op!(bool => 1 >= 1 = true);
    test_float_op!(bool => 0 >= 2 = false);
}

#[test]
fn test_const_collections() {
    let object = rune!(runestick::Object => fn main() { VALUE } const VALUE = #{};);
    assert!(object.is_empty());

    let tuple = rune!(runestick::Tuple => fn main() { VALUE } const VALUE = (););
    assert!(tuple.is_empty());

    let tuple = rune!(runestick::Tuple => fn main() { VALUE } const VALUE = ("Hello World",););
    assert_eq!(
        Some("Hello World"),
        tuple.get_value::<String>(0).unwrap().as_deref()
    );

    let vec = rune!(runestick::Vec => fn main() { VALUE } const VALUE = [];);
    assert!(vec.is_empty());

    let vec = rune!(runestick::Vec => fn main() { VALUE } const VALUE = ["Hello World"];);
    assert_eq!(
        Some("Hello World"),
        vec.get_value::<String>(0).unwrap().as_deref()
    );
}

#[test]
fn test_more_complexity() {
    let result = rune! { i64 =>
        const BASE = 10;
        const LIMIT = 0b1 << 10;

        const VALUE = {
            let timeout = BASE;

            while timeout < LIMIT {
                timeout *= 2;
            }

            timeout
        };

        fn main() { VALUE }
    };

    assert_eq!(result, 1280);
}

#[test]
fn test_if_else() {
    let result = rune! { i64 =>
        const VALUE = { if true { 1 } else if true { 2 } else { 3 } };
        fn main() { VALUE }
    };
    assert_eq!(result, 1);

    let result = rune! { i64 =>
        const VALUE = { if false { 1 } else if true { 2 } else { 3 } };
        fn main() { VALUE }
    };
    assert_eq!(result, 2);

    let result = rune! { i64 =>
        const VALUE = { if false { 1 } else if false { 2 } else { 3 } };
        fn main() { VALUE }
    };
    assert_eq!(result, 3);
}

#[test]
fn test_const_fn() {
    let result = rune! { i64 =>
        const VALUE = 2;
        const fn foo(n) { n + VALUE }

        fn main() {
            const VALUE = 1;
            foo(1 + 4 / 2 - VALUE) + foo(VALUE - 1)
        }
    };

    assert_eq!(result, 6);

    let result = rune_s! { String => r#"
    const VALUE = "baz";

    const fn foo(n) {
        `foo {n}`
    }
    
    fn main() {
        foo(`bar {VALUE}`)
    }
    "#};

    assert_eq!(result, "foo bar baz");

    let result = rune_s! { String => r#"
        const VALUE = foo("bar", "baz");

        const fn foo(a, b) {
            `foo {a} {b} {bar("biz")}`
        }
        
        const fn bar(c) {
            c
        }
        
        fn main() {
            VALUE
        }    
    "#};

    assert_eq!(result, "foo bar baz biz");
}

#[test]
fn test_const_fn_visibility() {
    let result = rune! { i64 =>
        pub mod a {
            pub mod b {
                pub const fn out(n) {
                    n + A
                }

                const A = 1;
            }
        }

        mod b {
            pub(super) fn out() {
                crate::a::b::out(B)
            }

            const B = 2;
        }

        fn main() {
            b::out()
        }
    };

    assert_eq!(result, 3);
}

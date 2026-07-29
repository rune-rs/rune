prelude!();

macro_rules! test_op {
    ($ty:ty => $lhs:literal $op:tt $rhs:literal = $result:literal) => {{
        let program = format!(
            r#"const A = {lhs}; const B = {rhs}; const VALUE = A {op} B; VALUE"#,
            lhs = $lhs,
            rhs = $rhs,
            op = stringify!($op),
        );

        let out: $ty = eval(&program);

        assert_eq!(
            out, $result,
            concat!("expected ", stringify!($result), " out of program `{}`"),
            program
        );
    }};
}

#[test]
fn test_const_values() {
    let out: bool = rune!(const VALUE = true; VALUE);
    assert_eq!(out, true);

    let out: String = rune!(const VALUE = "Hello World"; VALUE);
    assert_eq!(out, "Hello World");

    let out: String = eval(
        r#"
        const VALUE = `Hello ${WORLD} ${A} ${B} ${C}`;
        const WORLD = "World";
        const A = 1;
        const B = 1.0;
        const C = true;
        VALUE
    "#,
    );
    assert_eq!(out, "Hello World 1 1.0 true");
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
            r#"const A = {lhs}.0; const B = {rhs}.0; const VALUE = A {op} B; VALUE"#,
            lhs = $lhs,
            rhs = $rhs,
            op = stringify!($op),
        );

        let out: $ty = eval(&program);

        assert_eq!(
            out, $result,
            concat!("expected ", stringify!($result), " out of program `{}`"),
            program
        );
    }};
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
    let object: Object = rune!(fn main() { VALUE } const VALUE = #{}; main());
    assert!(object.is_empty());

    let tuple: Box<Tuple> = rune!(fn main() { VALUE } const VALUE = (); main());
    assert!(tuple.is_empty());

    let tuple: Box<Tuple> = rune!(fn main() { VALUE } const VALUE = ("Hello World",); main());
    assert_eq!(
        Some("Hello World"),
        tuple.get_value::<String>(0).unwrap().as_deref()
    );

    let vec: runtime::Vec = rune!(fn main() { VALUE } const VALUE = []; main());
    assert!(vec.is_empty());

    let vec: runtime::Vec = rune!(fn main() { VALUE } const VALUE = ["Hello World"]; main());
    assert_eq!(
        Some("Hello World"),
        vec.get_value::<String>(0).unwrap().as_deref()
    );
}

#[test]
fn test_more_complexity() {
    let result: i64 = rune! {
        const BASE = 10;
        const LIMIT = 0b1 << 10;

        const VALUE = {
            let timeout = BASE;

            while timeout < LIMIT {
                timeout *= 2;
            }

            timeout
        };

        VALUE
    };

    assert_eq!(result, 1280);
}

#[test]
fn test_if_else() {
    let result: i64 = rune! {
        const VALUE = { if true { 1 } else if true { 2 } else { 3 } };
        VALUE
    };
    assert_eq!(result, 1);

    let result: i64 = rune! {
        const VALUE = { if false { 1 } else if true { 2 } else { 3 } };
        VALUE
    };
    assert_eq!(result, 2);

    let result: i64 = rune! {
        const VALUE = { if false { 1 } else if false { 2 } else { 3 } };
        VALUE
    };
    assert_eq!(result, 3);
}

#[test]
fn test_const_fn() {
    let result: i64 = rune! {
        const VALUE = 2;
        const fn foo(n) { n + VALUE }

        fn bar() {
            const VALUE = 1;
            foo(1 + 4 / 2 - VALUE) + foo(VALUE - 1)
        }

        bar()
    };

    assert_eq!(result, 6);

    let result: String = eval(
        r#"
    const VALUE = "baz";

    const fn foo(n) {
        `foo ${n}`
    }

    foo(`bar ${VALUE}`)
    "#,
    );

    assert_eq!(result, "foo bar baz");

    let result: String = eval(
        r#"
        const VALUE = foo("bar", "baz");

        const fn foo(a, b) {
            `foo ${a} ${b} ${bar("biz")}`
        }

        const fn bar(c) {
            c
        }

        VALUE
    "#,
    );

    assert_eq!(result, "foo bar baz biz");
}

#[test]
fn test_const_fn_visibility() {
    let result: i64 = rune! {
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

        b::out()
    };

    assert_eq!(result, 3);
}

#[test]
fn test_const_block() {
    let result: i64 = rune! {
        let u = 2;
        let value = const { 1 << test() };
        return value - u;
        const fn test() { 32 }
    };

    assert_eq!(result, (1i64 << 32) - 2);

    let result: String = rune! {
        let var = "World";
        format!(const { "Hello {}" }, var)
    };

    assert_eq!(result, "Hello World");

    let result: String = rune! {
        let var = "World";
        return format!(const { FORMAT }, var);
        const FORMAT = "Hello {}";
    };

    assert_eq!(result, "Hello World");
}

/// A constant function called from a loop inside another constant is assembled
/// once and called repeatedly.
#[test]
fn test_const_fn_in_loop() {
    let result: i64 = rune! {
        const fn double(n) { n * 2 }

        const VALUE = {
            let n = 1;
            let i = 0;

            while i < 10 {
                n = double(n);
                i += 1;
            }

            n
        };

        VALUE
    };

    assert_eq!(result, 1024);
}

/// A constant function runs in a virtual machine, so it can construct and match
/// user defined types while it computes its value, even though those types
/// cannot be constant values themselves.
#[test]
fn test_const_fn_uses_user_types() {
    let result: i64 = rune! {
        enum Op { Add, Mul(a, b) }
        struct Point { x, y }

        const fn evaluate() {
            let op = Op::Mul(6, 7);
            let p = Point { x: 1, y: 2 };

            let n = match op {
                Op::Add => 0,
                Op::Mul(a, b) => a * b,
            };

            n + p.x + p.y
        }

        const VALUE = evaluate();
        VALUE
    };

    assert_eq!(result, 45);
}

/// A constant function is assembled once and called in a virtual machine, so it
/// can call itself. The recursion costs virtual machine call frames rather than
/// compiler stack frames.
#[test]
fn test_const_fn_recursion() {
    let result: i64 = rune! {
        const fn fib(n) {
            if n < 2 {
                n
            } else {
                fib(n - 1) + fib(n - 2)
            }
        }

        const VALUE = fib(15);
        VALUE
    };

    assert_eq!(result, 610);
}

/// Two constant functions which call each other are both assembled before
/// either is called, so their mutual recursion resolves.
#[test]
fn test_const_fn_mutual_recursion() {
    let result: bool = rune! {
        const fn is_even(n) {
            if n == 0 {
                true
            } else {
                is_odd(n - 1)
            }
        }

        const fn is_odd(n) {
            if n == 0 {
                false
            } else {
                is_even(n - 1)
            }
        }

        const VALUE = is_even(10);
        VALUE
    };

    assert_eq!(result, true);
}

prelude!();

use ErrorKind::*;
use VmErrorKind::*;

#[test]
fn test_bad_pattern() {
    // Attempting to assign to an unmatched pattern leads to a panic.
    assert_vm_error!(
        "let [] = [1, 2, 3];",
        Panic { reason } => {
            assert_eq!(reason.to_string(), "pattern did not match");
        }
    );
}

#[test]
fn test_const_in_pattern() {
    macro_rules! test_case_s {
        ($pat1:expr, $pat2:expr) => {
            let string = format!(
                r#"
                const PAT1 = {pat1};
                const PAT2 = {pat2};
                const PAT3 = {{
                    if true {{
                        {pat2}
                    }} else {{
                        {pat1}
                    }}
                }};

                let value = {pat1};
                let a = match value {{ PAT => 1, _ => 5 }};
                let b = match value {{ PAT2 => 2, _ => 6 }};
                let c = match value {{ PAT3 => 3, _ => 7 }};
                let d = match value {{ PAT4 => 4, _ => 8 }};
                (a, b, c, d)
                "#,
                pat1 = $pat1,
                pat2 = $pat2,
            );

            let tuple: (i64, i64, i64, i64) = eval(string);
            assert_eq!(tuple, (1, 6, 7, 4));
        };
    }

    macro_rules! test_case {
        ($pat1:expr, $pat2:expr) => {
            test_case_s!(stringify!($pat1), stringify!($pat2))
        };
    }

    test_case!(true, false);
    test_case!('a', 'b');
    test_case!(b'a', b'b');
    test_case!(10, 20);
    test_case!("abc", "def");
    test_case!(b"abc", b"def");
    test_case!((1, 2), (3, 4));
    test_case!([1, 2], [3, 4]);
    test_case!([1, (3, 4)], [3, (3, 4)]);
    test_case_s!("#{foo: 12}", "#{bar: 12}");
    test_case_s!("#{}", "#{bar: 12}");

    assert_errors! {
        r#"
        const PAT = 3.1415;

        pub fn main() {
            let value = 3.1415;
            match value { PAT => true, _ => false }
        }
        "#,
        span!(112, 115), Custom { error } => {
            assert_eq!(error.to_string(), "Unsupported constant value in pattern");
        }
    }
}

#[test]
fn test_or_pattern() {
    // An or-pattern matches when the value equals any of its alternatives, and
    // falls through to the next arm when it equals none of them.
    let out: (i64, i64, i64, i64) = eval(
        r#"
        fn classify(n) {
            match n {
                1 | 2 | 3 => 1,
                _ => 0,
            }
        }
        (classify(1), classify(2), classify(3), classify(4))
        "#,
    );
    assert_eq!(out, (1, 1, 1, 0));
}

#[test]
fn test_or_pattern_strings() {
    let out: (i64, i64) = eval(
        r#"
        fn f(s) { match s { "a" | "b" => 1, _ => 0 } }
        (f("b"), f("z"))
        "#,
    );
    assert_eq!(out, (1, 0));
}

#[test]
fn test_or_pattern_with_guard() {
    let out: (i64, i64) = eval(
        r#"
        fn f(n) {
            match n {
                1 | 2 if n > 1 => 2,
                1 | 2 => 1,
                _ => 0,
            }
        }
        (f(1), f(2))
        "#,
    );
    assert_eq!(out, (1, 2));
}

#[test]
fn test_or_pattern_multiple_arms() {
    let out: i64 = eval(
        r#"
        match 7 {
            1 | 2 => 10,
            6 | 7 | 8 => 20,
            _ => 30,
        }
        "#,
    );
    assert_eq!(out, 20);
}

#[test]
fn test_or_pattern_binding_rejected() {
    // Every alternative binds no variables, since there would be no single
    // value for the arm body to read a binding back from.
    assert_errors! {
        "pub fn main() { match 1 { a | b => 1, _ => 0 } }",
        _, OrPatternBinding
    };

    assert_errors! {
        "pub fn main() { match 1 { 1 | x => 1, _ => 0 } }",
        _, OrPatternBinding
    };
}

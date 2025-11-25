//! Edge case tests for gradual typing.
//!
//! Tests boundary conditions and unusual but valid scenarios.

prelude!();

/// Test unit tuple type ()
#[test]
fn edge_case_unit_tuple() {
    let _: () = rune! {
        fn returns_unit() -> () {
            ()
        }

        pub fn main() {
            returns_unit();
        }
    };
}

/// Test single-element tuples (need trailing comma)
#[test]
fn edge_case_single_element_tuple() {
    let result: (i64,) = rune! {
        fn single_tuple() -> (i64,) {
            (42,)
        }

        single_tuple()
    };
    assert_eq!(result.0, 42);
}

/// Test empty struct
#[test]
fn edge_case_empty_struct() {
    let _: () = rune! {
        struct Empty {}

        pub fn main() {
            let e = Empty {};
        }
    };
}

/// Test never type (!)
#[test]
fn edge_case_never_type() {
    // Never type in type annotations should be recognized
    assert_no_type_diagnostics! {
        r#"
        fn never_returns() -> ! {
            loop {}
        }
        "#
    };
}

/// Test EXTREME: maximum nested tuples (7 levels - near recursion limit)
#[test]
fn edge_case_maximum_tuple_nesting() {
    let _: () = rune! {
        pub fn main() {
            let x = (((((((42,),),),),),),);
            let a = x.0;
            let b = a.0;
            let c = b.0;
            let d = c.0;
            let e = d.0;
            let f = e.0;
            assert_eq!(f.0, 42);
        }
    };
}

/// Test MINIMAL: zero-parameter function
#[test]
fn edge_case_zero_parameters() {
    let result: i64 = rune! {
        fn no_params() -> i64 {
            42
        }
        no_params()
    };
    assert_eq!(result, 42);
}

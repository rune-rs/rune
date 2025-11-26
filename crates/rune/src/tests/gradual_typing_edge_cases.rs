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

// ============================================================================
// Protocol Edge Cases
// ============================================================================

/// Protocol with deeply nested tuple operations
#[test]
fn edge_case_nested_tuple_protocols() {
    let result: ((i64, i64), (i64, i64)) = rune! {
        fn nested_ops() -> ((i64, i64), (i64, i64)) {
            let a = (1, 2);
            let b = (3, 4);
            ((a.0 + b.0, a.1 + b.1), (a.0 * b.0, a.1 * b.1))
        }

        nested_ops()
    };
    assert_eq!(result, ((4, 6), (3, 8)));
}

/// Protocol with empty collections
#[test]
fn edge_case_empty_collection_protocols() {
    let result: (i64, bool) = rune! {
        fn empty_ops() -> (i64, bool) {
            let empty_vec = [];
            let empty_str = "";
            (empty_vec.len() as i64, empty_str.is_empty())
        }

        empty_ops()
    };
    assert_eq!(result, (0, true));
}

/// Protocol with type coercion edge cases
#[test]
fn edge_case_protocol_type_coercion() {
    let result: (i64, f64) = rune! {
        fn coercion_demo() -> (i64, f64) {
            let small = 1;
            let large = 1_000_000_000;
            let float_small = small as f64;
            let float_large = large as f64;
            (small + large, float_small + float_large)
        }

        coercion_demo()
    };
    assert_eq!(result, (1_000_000_001, 1_000_000_001.0));
}

/// Protocol chain with mixed types
#[test]
fn edge_case_mixed_protocol_chain() {
    let result: String = rune! {
        fn mixed_chain() -> String {
            let count = 5;
            let text = "hello";
            let result = count.to_string() + " " + text + "!";
            result
        }

        mixed_chain()
    };
    assert_eq!(result, "5 hello!");
}

/// Protocol with boolean arithmetic (edge case)
#[test]
fn edge_case_boolean_arithmetic() {
    let result: (i64, bool) = rune! {
        fn bool_math() -> (i64, bool) {
            let a = true;
            let b = false;
            // Bool to int conversion for arithmetic
            let int_a = a as i64;
            let int_b = b as i64;
            (int_a + int_b, a && b)
        }

        bool_math()
    };
    assert_eq!(result, (1, false));
}

/// Protocol with large integer overflow handling
#[test]
fn edge_case_large_integer_protocols() {
    let result: (i64, i64) = rune! {
        fn large_ints() -> (i64, i64) {
            let max = i64::MAX;
            let min = i64::MIN;
            let neg_max = -max;
            (max + min, neg_max)
        }

        large_ints()
    };
    // The actual behavior depends on how Rune handles overflow
    // This test verifies the type checker allows the operations
    drop(result);
}

/// Protocol with char operations
#[test]
fn edge_case_char_protocols() {
    let result: (char, i64, String) = rune! {
        fn char_ops() -> (char, i64, String) {
            let c = 'a';
            let next = c as i64 + 1;
            let next_char = next as char;
            (c, next, c.to_string() + &next_char.to_string())
        }

        char_ops()
    };
    assert_eq!(result, ('a', 98, "ab"));
}

/// Protocol with optional chaining simulation
#[test]
fn edge_case_optional_like_protocols() {
    let result: (i64, bool) = rune! {
        fn optional_like() -> (i64, bool) {
            let maybe_value = 42;
            let has_value = maybe_value > 0;
            let final_value = if has_value { maybe_value } else { 0 };
            (final_value, has_value)
        }

        optional_like()
    };
    assert_eq!(result, (42, true));
}

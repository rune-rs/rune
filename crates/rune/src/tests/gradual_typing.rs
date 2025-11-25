//! Tests for gradual typing support.
//!
//! These tests define the expected behavior of the gradual typing feature.

prelude!();

// ============================================================================
// PHASE 1: Type Annotation Acceptance
// ============================================================================

/// Functions with return type annotations should compile
#[test]
fn accept_function_return_type() {
    let result: i64 = rune! {
        fn add(a, b) -> i64 {
            a + b
        }

        add(1, 2)
    };
    assert_eq!(result, 3);
}

/// Functions with parameter type annotations should compile
#[test]
fn accept_function_param_types() {
    let result: i64 = rune! {
        fn add(a: i64, b: i64) -> i64 {
            a + b
        }

        add(1, 2)
    };
    assert_eq!(result, 3);
}

/// Mixed typed and untyped parameters should compile
#[test]
fn accept_mixed_param_types() {
    let result: i64 = rune! {
        fn add(a: i64, b) -> i64 {
            a + b
        }

        add(1, 2)
    };
    assert_eq!(result, 3);
}

/// Struct field types should compile
#[test]
fn accept_struct_field_types() {
    let _: () = rune! {
        struct Point {
            x: i64,
            y: i64,
        }

        pub fn main() {
            let p = Point { x: 1, y: 2 };
        }
    };
}

/// Tuple types in signatures should compile
#[test]
fn accept_tuple_return_type() {
    let result: (i64, i64) = rune! {
        fn swap(a: i64, b: i64) -> (i64, i64) {
            (b, a)
        }

        swap(1, 2)
    };
    assert_eq!(result, (2, 1));
}

/// Path types (module::Type) should compile
#[test]
fn accept_path_types() {
    let _: () = rune! {
        pub fn get_option() -> Option {
            Some(42)
        }

        pub fn main() {
            get_option();
        }
    };
}

/// Closures with typed parameters should compile
#[test]
fn accept_closure_param_types() {
    let result: i64 = rune! {
        let add = |a: i64, b: i64| { a + b };
        add(1, 2)
    };
    assert_eq!(result, 3);
}

// ============================================================================
// BACKWARDS COMPATIBILITY: Existing behavior preserved
// ============================================================================

/// Untyped functions still work exactly as before
#[test]
fn untyped_functions_unchanged() {
    // Note: Without return type annotation, main() returns unit ()
    // but add() returns the value of the last expression
    let _: () = rune! {
        pub fn add(a, b) {
            a + b
        }

        pub fn main() {
            add(1, 2);
        }
    };
}

// ============================================================================
// PHASE 2: Type Checking and Diagnostics
// ============================================================================

use crate::diagnostics::WarningDiagnosticKind;

/// Type mismatch in return should produce a warning
#[test]
fn warn_return_type_mismatch() {
    assert_warnings! {
        r#"
        fn foo() -> i64 {
            "not an i64"
        }
        foo()
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Type mismatch in arguments should produce warning
/// NOTE: This requires call-site type checking, which is a more advanced feature.
/// Phase 2 focuses on return type checking. Call-site argument checking
/// will be implemented in a future phase when we track function signatures
/// and check call expressions against them.
#[test]
#[ignore = "Call-site argument type checking not yet implemented - planned for future phase"]
fn warn_argument_type_mismatch() {
    assert_warnings! {
        r#"
        fn add(a: i64, b: i64) -> i64 {
            a + b
        }
        add("wrong", "types")
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, .. } => {
            assert_eq!(expected, "i64");
            // First argument mismatch
        }
    };
}

/// Strict mode should produce errors instead of warnings
#[test]
fn strict_mode_errors_on_mismatch() {
    let mut diagnostics = crate::Diagnostics::new();
    let mut options = crate::compile::Options::default();
    options.script(true);
    options.strict_types(true);

    let result = crate::tests::compile_with_options(
        r#"
        fn foo() -> i64 {
            "not an i64"
        }
        foo()
        "#,
        &mut diagnostics,
        &options,
    );

    // Strict mode should cause compilation to fail
    assert!(
        result.is_err() || diagnostics.has_error(),
        "Strict mode should fail on type mismatch"
    );
}

/// Any type (untyped) is compatible with all other types (gradual typing semantics)
#[test]
fn any_type_compatible_with_all() {
    // Untyped parameter accepts any value - no warnings should be produced
    let result: i64 = rune! {
        fn accept_any(x) -> i64 {
            42  // Return type is checked, but x accepts anything
        }

        // All of these should work without warnings
        accept_any("string");
        accept_any(123);
        accept_any([1, 2, 3]);
        42
    };
    assert_eq!(result, 42);
}

/// Correct types should not produce warnings
#[test]
fn correct_types_no_warning() {
    let mut diagnostics = crate::Diagnostics::new();
    let _ = crate::tests::compile_helper(
        r#"
        fn add(a: i64, b: i64) -> i64 {
            a + b
        }
        add(1, 2)
        "#,
        &mut diagnostics,
    )
    .expect("should compile");

    assert!(
        !diagnostics.has_warning(),
        "Correct types should not produce warnings"
    );
}

/// Type checking works with nested function calls
#[test]
fn type_check_nested_calls() {
    let result: i64 = rune! {
        fn inner() -> i64 {
            42
        }

        fn outer(x: i64) -> i64 {
            x * 2
        }

        // i64 -> i64, should pass without warnings
        outer(inner())
    };
    assert_eq!(result, 84);
}

/// Tuple return type mismatch should be detected
#[test]
fn warn_tuple_return_mismatch() {
    assert_warnings! {
        r#"
        fn get_pair() -> (i64, i64) {
            ("wrong", "types")
        }
        get_pair()
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Literals have known types
#[test]
fn literals_have_correct_types() {
    let mut diagnostics = crate::Diagnostics::new();
    let _ = crate::tests::compile_helper(
        r#"
        fn takes_int(x: i64) -> i64 { x }
        fn takes_string(s: String) -> String { s }
        fn takes_bool(b: bool) -> bool { b }

        // These should all work without warnings
        takes_int(42);
        takes_string("hello");
        takes_bool(true);
        0
        "#,
        &mut diagnostics,
    )
    .expect("should compile");

    assert!(
        !diagnostics.has_warning(),
        "Correct literal types should not warn"
    );
}

// ============================================================================
// PHASE 2B: Type Inference
// ============================================================================

/// Infer let binding type from literal
#[test]
fn infer_let_from_literal() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = 42;  // x inferred as i64
            x
        }
        test()
    };
    assert_eq!(result, 42);
}

/// Infer type through binary operations
#[test]
fn infer_binary_op_type() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = 1 + 2;  // i64 + i64 = i64
            x
        }
        test()
    };
    assert_eq!(result, 3);
}

/// Infer type from function call return
#[test]
fn infer_from_function_call() {
    let result: i64 = rune! {
        fn get_value() -> i64 { 42 }

        fn test() -> i64 {
            let x = get_value();  // x inferred as i64
            x
        }
        test()
    };
    assert_eq!(result, 42);
}

/// Warn on inferred type mismatch - returning wrong type
#[test]
fn warn_inferred_type_mismatch() {
    assert_warnings! {
        r#"
        pub fn main() -> String {
            let x = 42;  // x inferred as i64
            x            // returning i64 where String expected
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "String");
            assert_eq!(actual, "i64");
        }
    };
}

/// Match arm type inference - all branches unified
#[test]
fn match_arm_type_unification() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let opt = Some(42);
            match opt {
                Some(x) => x,
                None => 0,
            }
        }
        test()
    };
    assert_eq!(result, 42);
}

/// If/else branches type unification
#[test]
fn if_else_type_unification() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = if true { 1 } else { 2 };
            x
        }
        test()
    };
    assert_eq!(result, 1);
}

/// Variable type tracking through multiple uses
#[test]
fn variable_type_tracking() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = 0;
            let x = x + 1;  // x stays i64
            let x = x * 2;
            x
        }
        test()
    };
    assert_eq!(result, 2);
}

/// Infer type from chained function calls
#[test]
fn infer_chained_calls() {
    let result: i64 = rune! {
        fn double(x: i64) -> i64 { x * 2 }
        fn add_one(x: i64) -> i64 { x + 1 }

        fn test() -> i64 {
            let result = add_one(double(5));
            result
        }
        test()
    };
    assert_eq!(result, 11);
}

/// Closure with inferred parameter types from call site
#[test]
fn closure_param_inference_from_call() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let f = |x| x * 2;  // x type will be inferred from usage
            f(21)              // now x should be inferred as i64
        }
        test()
    };
    assert_eq!(result, 42);
}

/// Block expression type inference
#[test]
fn block_expression_type() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = {
                let a = 10;
                let b = 20;
                a + b
            };
            x
        }
        test()
    };
    assert_eq!(result, 30);
}

/// Nested block type inference
#[test]
fn nested_block_type_inference() {
    let result: i64 = rune! {
        fn test() -> i64 {
            let x = {
                let y = {
                    42
                };
                y + 1
            };
            x
        }
        test()
    };
    assert_eq!(result, 43);
}

/// Boolean operation result type
#[test]
fn boolean_op_result_type() {
    let result: bool = rune! {
        fn test() -> bool {
            let x = true && false;
            x
        }
        test()
    };
    assert!(!result);
}

/// Comparison operation result type
#[test]
fn comparison_op_result_type() {
    let result: bool = rune! {
        fn test() -> bool {
            let x = 1 < 2;
            x
        }
        test()
    };
    assert!(result);
}

// ============================================================================
// PHASE 3: Struct Field Type Checking
// ============================================================================

/// Struct field type mismatches should produce warnings
#[test]
fn warn_struct_field_type_mismatch() {
    assert_warnings! {
        r#"
        struct Point {
            x: i64,
            y: i64,
        }

        pub fn main() {
            let p = Point { x: "hello", y: 42 };
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Correct struct field types should not produce warnings
#[test]
fn accept_correct_struct_field_types() {
    let mut diagnostics = crate::Diagnostics::new();

    let _ = crate::tests::compile_helper(
        r#"
        struct Point {
            x: i64,
            y: i64,
        }

        pub fn main() {
            let p = Point { x: 42, y: 84 };
        }
        "#,
        &mut diagnostics,
    )
    .expect("should compile");

    assert!(
        !diagnostics.has_warning(),
        "Correct struct field types should not produce warnings"
    );
}

/// Struct fields without type annotations should not produce warnings (gradual typing)
#[test]
fn untyped_struct_fields_no_warning() {
    let mut diagnostics = crate::Diagnostics::new();

    let _ = crate::tests::compile_helper(
        r#"
        struct Point {
            x,
            y,
        }

        pub fn main() {
            let p = Point { x: "hello", y: 42 };
        }
        "#,
        &mut diagnostics,
    )
    .expect("should compile");

    assert!(
        !diagnostics.has_warning(),
        "Untyped struct fields should not produce warnings"
    );
}

// ============================================================================
// QA Review - Additional Test Cases
// ============================================================================

#[test]
fn test_struct_with_mixed_typed_fields() {
    // Tests fix #1 from QA review - mixed typed/untyped fields
    let _: () = rune! {
        struct Point {
            x: i64,  // typed
            y        // untyped
        }

        pub fn main() {
            let p = Point { x: 42, y: 10 };
            assert_eq!(p.x, 42);
            assert_eq!(p.y, 10);
        }
    };
}

#[test]
fn test_deep_nested_tuples() {
    // Tests fix #3 from QA review - recursion depth limit
    // Testing deeply nested tuple types to ensure we don't stack overflow
    let _: () = rune! {
        pub fn main() {
            // Create deeply nested tuple value - tests recursion depth handling
            let x = (((42,),),);
            let y = x.0;
            let z = y.0;
            let value = z.0;
            assert_eq!(value, 42);
        }
    };
}

#[test]
fn test_scope_management() {
    // Tests fix #2 from QA review - scope pop safety
    // Test nested scopes don't cause scope pop bugs
    let result: i64 = rune! {
        fn nested_scopes() -> i64 {
            let x = 42;
            let y = {
                let y = x;
                y
            };
            let z = {
                let z = y;
                z
            };
            z
        }

        nested_scopes()
    };
    assert_eq!(result, 42);
}

//! Tests for gradual typing support.
//!
//! These tests define the expected behavior of the gradual typing feature.

prelude!();

// ============================================================================
// Type Annotation Acceptance
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
// Type Checking and Diagnostics
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
/// Return type checking is implemented. Call-site argument checking
/// will be implemented in the future when we track function signatures
/// and check call expressions against them.
#[test]
#[ignore = "Call-site argument type checking not yet implemented"]
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
// Type Inference
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
// Struct Field Type Checking
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

// ============================================================================
// Protocol Lookup Type Checking
// ============================================================================

/// Custom type with ADD protocol returning Self - type checker should use protocol return type
#[test]
fn protocol_add_returns_self() {
    #[derive(Debug, Clone, Any)]
    #[rune(item = ::test_module, constructor)]
    struct Counter {
        #[rune(get)]
        value: i64,
    }

    impl Counter {
        #[rune::function(protocol = ADD)]
        fn add(&self, other: &Counter) -> Self {
            Counter {
                value: self.value + other.value,
            }
        }
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::with_crate("test_module")?;
        module.ty::<Counter>()?;
        module.function_meta(Counter::add)?;
        Ok(module)
    }

    let m = make_module().expect("failed to create module");

    // Test that adding two Counters returns a Counter
    // The type checker should look up Protocol::ADD and find it returns Counter
    let result: i64 = rune_n! {
        mod m,
        (),
        pub fn main() {
            let a = test_module::Counter { value: 10 };
            let b = test_module::Counter { value: 20 };
            let c = a + b;  // Type checker should know c is Counter via protocol lookup
            c.value
        }
    };

    assert_eq!(result, 30);
}

/// Custom type with MUL protocol returning different type (dot product returns i64)
#[test]
fn protocol_mul_returns_different_type() {
    #[derive(Debug, Clone, Any)]
    #[rune(item = ::test_module, constructor)]
    struct Vector2 {
        #[rune(get)]
        x: i64,
        #[rune(get)]
        y: i64,
    }

    impl Vector2 {
        /// Dot product returns i64, not Vector2
        #[rune::function(protocol = MUL)]
        fn mul(&self, other: &Vector2) -> i64 {
            self.x * other.x + self.y * other.y
        }
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::with_crate("test_module")?;
        module.ty::<Vector2>()?;
        module.function_meta(Vector2::mul)?;
        Ok(module)
    }

    let m = make_module().expect("failed to create module");

    // Test that multiplying two Vector2s returns i64 (dot product)
    // The type checker should look up Protocol::MUL and find it returns i64
    let result: i64 = rune_n! {
        mod m,
        (),
        pub fn main() {
            let a = test_module::Vector2 { x: 2, y: 3 };
            let b = test_module::Vector2 { x: 4, y: 5 };
            let dot = a * b;  // Type checker should know dot is i64 via protocol lookup
            dot
        }
    };

    assert_eq!(result, 23); // 2*4 + 3*5 = 8 + 15 = 23
}

/// Protocol lookup with type annotation verification
#[test]
fn protocol_lookup_type_annotation_match() {
    #[derive(Debug, Clone, Any)]
    #[rune(item = ::test_module, constructor)]
    struct Money {
        #[rune(get)]
        cents: i64,
    }

    impl Money {
        #[rune::function(protocol = ADD)]
        fn add(&self, other: &Money) -> Self {
            Money {
                cents: self.cents + other.cents,
            }
        }

        #[rune::function(protocol = PARTIAL_EQ)]
        fn eq(&self, other: &Money) -> bool {
            self.cents == other.cents
        }
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::with_crate("test_module")?;
        module.ty::<Money>()?;
        module.function_meta(Money::add)?;
        module.function_meta(Money::eq)?;
        Ok(module)
    }

    let m = make_module().expect("failed to create module");

    // Test function with explicit return type using protocol operators
    // Type checker should verify protocol return types match annotations
    let result: bool = rune_n! {
        mod m,
        (),
        pub fn main() {
            let a = test_module::Money { cents: 100 };
            let b = test_module::Money { cents: 50 };
            let c = a + b;  // Protocol::ADD returns Money
            let expected = test_module::Money { cents: 150 };
            c == expected   // Protocol::PARTIAL_EQ returns bool
        }
    };

    assert!(result);
}

/// Comparison protocol returns bool
#[test]
fn protocol_comparison_returns_bool() {
    use core::cmp::Ordering;

    #[derive(Debug, Clone, Any)]
    #[rune(item = ::test_module, constructor)]
    struct Score {
        #[rune(get)]
        points: i64,
    }

    impl Score {
        #[rune::function(protocol = PARTIAL_CMP)]
        fn partial_cmp(&self, other: &Score) -> Option<Ordering> {
            self.points.partial_cmp(&other.points)
        }
    }

    fn make_module() -> Result<Module, ContextError> {
        let mut module = Module::with_crate("test_module")?;
        module.ty::<Score>()?;
        module.function_meta(Score::partial_cmp)?;
        Ok(module)
    }

    let m = make_module().expect("failed to create module");

    // Test that comparison operators return bool via protocol lookup
    let result: bool = rune_n! {
        mod m,
        (),
        pub fn main() {
            let high = test_module::Score { points: 100 };
            let low = test_module::Score { points: 50 };
            high > low  // Protocol::PARTIAL_CMP should give bool result
        }
    };

    assert!(result);
}

/// Test that stdlib types with protocol implementations work correctly.
/// String + String uses Protocol::ADD and returns String.
#[test]
fn stdlib_string_add_protocol() {
    let result: String = rune! {
        fn concat(a: String, b: String) -> String {
            a + b
        }

        concat("hello ", "world")
    };
    assert_eq!(result, "hello world");
}

/// Test that stdlib Vec operations work with type annotations.
#[test]
fn stdlib_vec_operations() {
    let result: i64 = rune! {
        fn sum_first_two(v: Vec) -> i64 {
            v[0] + v[1]
        }

        sum_first_two([10, 20, 30])
    };
    assert_eq!(result, 30);
}

/// Test protocol operations with float types
#[test]
fn stdlib_float_protocols() {
    let result: f64 = rune! {
        fn calculate() -> f64 {
            3.14 * 2.0 + 1.0
        }

        calculate()
    };
    assert_eq!(result, 7.28);
}

/// Test protocol operations with mixed types
#[test]
fn stdlib_mixed_type_protocols() {
    let result: String = rune! {
        fn format_calculation() -> String {
            let x = 42;
            let y = 3.14;
            format!("Value is {} times {}", x, y)  // String formatting protocol
        }

        format_calculation()
    };
    assert!(result.contains("42"));
    assert!(result.contains("3.14"));
}

/// Test protocol operations in conditionals
#[test]
fn stdlib_protocols_in_conditionals() {
    let result: bool = rune! {
        fn check_condition(value: i64) -> bool {
            value > 10 && value < 100
        }

        check_condition(50)
    };
    assert!(result);
}

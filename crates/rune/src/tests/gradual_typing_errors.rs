//! Error handling tests for gradual typing.
//!
//! Tests error messages and diagnostic quality.

prelude!();

use crate::diagnostics::WarningDiagnosticKind;

/// Basic type mismatch produces clear warning
#[test]
fn error_basic_type_mismatch() {
    assert_warnings! {
        r#"
        fn get_number() -> i64 {
            "not a number"
        }
        get_number()
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Struct field type mismatch
#[test]
fn error_struct_field_mismatch() {
    assert_warnings! {
        r#"
        struct Point {
            x: i64,
            y: i64,
        }

        pub fn main() {
            let p = Point { x: "wrong", y: 42 };
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Tuple type mismatch
#[test]
fn error_tuple_mismatch() {
    assert_warnings! {
        r#"
        fn get_pair() -> (i64, i64) {
            ("wrong", 42)
        }
        get_pair()
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Strict mode converts warnings to errors
#[test]
fn error_strict_mode() {
    let mut diagnostics = crate::Diagnostics::new();
    let mut options = crate::compile::Options::default();
    options.script(true);
    options.strict_types(true);

    let result = crate::tests::compile_with_options(
        r#"
        fn bad() -> i64 {
            "not an i64"
        }
        bad()
        "#,
        &mut diagnostics,
        &options,
    );

    // Should fail in strict mode
    assert!(
        result.is_err() || diagnostics.has_error(),
        "Strict mode should convert type mismatches to errors"
    );
}

/// Multiple type mismatches in same function
#[test]
fn error_multiple_mismatches() {
    // Should produce multiple warnings
    let mut diagnostics = crate::Diagnostics::new();
    let _ = crate::tests::compile_helper(
        r#"
        struct Point {
            x: i64,
            y: i64,
            z: i64,
        }

        pub fn main() {
            let p = Point { x: "wrong", y: "also wrong", z: 42 };
        }
        "#,
        &mut diagnostics,
    );

    assert!(diagnostics.has_warning(), "Should have type mismatch warnings");
}

/// Nested type mismatch (tuple inside struct)
#[test]
fn error_nested_type_mismatch() {
    assert_warnings! {
        r#"
        struct Container {
            data: (i64, i64),
        }

        pub fn main() {
            let c = Container { data: ("wrong", 42) };
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

// ============================================================================
// Protocol-Related Errors
// ============================================================================

/// Protocol operation with incompatible types
#[test]
fn error_protocol_incompatible_types() {
    assert_warnings! {
        r#"
        fn add_string_and_number() -> i64 {
            "hello" + 42
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Protocol return type mismatch
#[test]
fn error_protocol_return_mismatch() {
    assert_warnings! {
        r#"
        fn multiply_returns_string() -> i64 {
            5 * 3.14  // This produces f64, not i64
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            // The actual type might be f64 or a generic number type
            assert!(actual.contains("f64") || actual.contains("float"));
        }
    };
}

/// Protocol comparison with different types
#[test]
fn error_protocol_comparison_mismatch() {
    assert_warnings! {
        r#"
        fn compare_string_and_number() -> bool {
            42 == "42"
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Protocol with wrong number of arguments
#[test]
fn error_protocol_wrong_arg_count() {
    assert_warnings! {
        r#"
        fn too_many_args() -> i64 {
            1 + 2 + 3 + 4 + 5  // ADD is binary, this chains correctly
        }
        "#,
        // This might not generate a warning as protocol chaining is valid
    };
}

/// Protocol with undefined operation
#[test]
fn error_protocol_undefined_operation() {
    assert_warnings! {
        r#"
        struct CustomType {}

        fn use_custom_in_math() -> CustomType {
            let a = CustomType {};
            let b = CustomType {};
            a + b  // CustomType doesn't implement ADD
        }
        "#,
        _span,
        WarningDiagnosticKind::MissingProtocol { .. } | WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Protocol in conditional with wrong type
#[test]
fn error_protocol_conditional_mismatch() {
    assert_warnings! {
        r#"
        fn conditional_arithmetic() -> i64 {
            let condition = true;
            if condition {
                10 + 20
            } else {
                "not a number"  // Type mismatch between branches
            }
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert_eq!(expected, "i64");
            assert_eq!(actual, "String");
        }
    };
}

/// Protocol with array index out of bounds (runtime error, type-check should allow)
#[test]
fn error_protocol_array_bounds() {
    let _: () = rune! {
        fn access_array() -> i64 {
            let arr = [1, 2, 3];
            arr[10]  // Type checker allows this, runtime error
        }

        pub fn main() {
            // Don't actually call it to avoid runtime error
        }
    };
}

/// Protocol with failed type conversion
#[test]
fn error_protocol_failed_conversion() {
    assert_warnings! {
        r#"
        fn convert_to_string() -> i64 {
            let x = "not a number";
            x as i64  // Invalid conversion
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. } | WarningDiagnosticKind::UnsupportedCast { .. }
    };
}

/// Protocol chaining with mismatched types
#[test]
fn error_protocol_chain_mismatch() {
    assert_warnings! {
        r#"
        fn chain_mismatch() -> String {
            let x = 10;
            let y = x + 5;  // y is i64
            y + "hello"     // Can't add String to i64
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { .. }
    };
}

/// Protocol with tuple access on non-tuple
#[test]
fn error_protocol_tuple_on_non_tuple() {
    assert_warnings! {
        r#"
        fn access_non_tuple() -> i64 {
            let x = 42;
            x.0  // x is not a tuple
        }
        "#,
        _span,
        WarningDiagnosticKind::TypeMismatch { expected, actual, .. } => {
            assert!(expected.contains("tuple") || expected.contains("struct"));
            assert_eq!(actual, "i64");
        }
    };
}

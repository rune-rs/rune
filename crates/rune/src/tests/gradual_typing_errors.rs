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
// Protocol-Related Tests
// ============================================================================
// Note: The gradual type checker currently focuses on explicit type annotations
// (function return types, struct fields). It doesn't check protocol operand
// compatibility or if/else branch unification. These are valid future enhancements.

/// Protocol with chained operations (valid - no errors expected)
#[test]
fn protocol_chaining_is_valid() {
    // Protocol chaining like 1 + 2 + 3 is valid and should not error
    assert_parse! {
        r#"
        fn chained_ops() -> i64 {
            1 + 2 + 3 + 4 + 5  // ADD is binary, this chains correctly
        }
        "#
    };
}

/// Protocol with undefined operation - compiles but may fail at runtime
#[test]
fn protocol_undefined_operation_compiles() {
    // CustomType without ADD protocol compiles fine (gradual typing allows this)
    // Would fail at runtime if called
    assert_parse! {
        r#"
        struct CustomType {}

        fn use_custom_in_math() -> CustomType {
            let a = CustomType {};
            let b = CustomType {};
            a + b  // CustomType doesn't implement ADD - runtime error, not compile error
        }
        "#
    };
}

/// Protocol with array index out of bounds (runtime error, type-check should allow)
#[test]
fn protocol_array_bounds_compiles() {
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

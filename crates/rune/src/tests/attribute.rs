prelude!();

use ErrorKind::*;

#[test]
fn basic_use() {
    rune! {
        mod private {
            #[test]
            fn test_case() {
                assert_eq!(1 + 1, 2);
            }
        }

        #[test]
        fn test_case() {
            assert_eq!(1 + 1, 2);
        }

        pub fn main() {
        }
    };
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_use() {
    assert_errors! {
        r#"
        fn function() {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        }
        "#,
        span, NestedTest { nested_span } => {
            assert_eq!(span, span!(37, 44));
            assert_eq!(nested_span, span!(9, 22));
        }
    }

    assert_errors! {
        r#"
        const ITEM = {
            #[test]
            fn test_fn() {
                assert!(true != true);
            }
        };
        "#,
        span!(36, 43), NestedTest { nested_span: span!(9, 19) }
    }
}

// We prevent tests from being declared inside of nested items at compile time.
#[test]
fn deny_nested_bench() {
    assert_errors! {
        r#"
        fn function() {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        }
        "#,
        span!(37, 71), NestedBench { nested_span: span!(9, 22) }
    }

    assert_errors! {
        r#"
        const ITEM = {
            #[bench]
            fn bench_fn() {
                assert!(true != true);
            }
        };
        "#,
        span!(36, 70), NestedBench { nested_span: span!(9, 19) }
    }
}

#[test]
fn deny_struct_attributes() {
    assert_errors! {
        "#[struct_attribute] struct Struct {}",
        span!(0, 19), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on structs are not supported");
        }
    }
}

#[test]
fn deny_enum_attributes() {
    assert_errors! {
        "#[enum_attribute] enum Enum {}",
        span!(0, 17), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on enums are not supported");
        }
    }
}

#[test]
fn deny_fn_attributes() {
    assert_errors! {
        "#[function_attribute] fn function() {}",
        span!(0, 21), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on functions are not supported");
        }
    }
}

#[test]
fn deny_const_attributes() {
    assert_errors! {
        "#[constant_attribute] const CONSTANT = 42;",
        span!(0, 21), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on constants are not supported");
        }
    }
}

#[test]
fn deny_use_attributes() {
    assert_errors! {
        "#[use_attribute] use std::str;",
        span!(0, 16), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on uses are not supported");
        }
    }
}

#[test]
fn deny_mod_attributes() {
    assert_errors! {
        "#[mod_attribute] mod inner {}",
        span!(0, 16), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on modules are not supported");
        }
    }
}

#[test]
fn deny_local_attributes() {
    assert_errors! {
        "pub fn main() { #[local_attribute] let x = 1; }",
        span!(16, 34), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on local declarations are not supported");
        }
    };
}

#[test]
fn deny_block_attributes() {
    assert_errors! {
        r#"pub fn main() { #[block_attribute] {} }"#,
        span!(16, 34), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on blocks are not supported");
        }
    };
}

#[test]
fn deny_macro_attributes() {
    assert_errors! {
        r#"#[macro_attribute] macro_call!()"#,
        span!(0, 18), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on macros are not supported");
        }
    };
}

#[test]
fn deny_field_attributes() {
    assert_errors! {
        r#"struct Struct { #[field_attribute] field }"#,
        span!(16, 34), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on fields are not supported");
        }
    };
}

#[test]
fn deny_variant_attributes() {
    assert_errors! {
        r#"enum Enum { #[field_attribute] Variant }"#,
        span!(12, 30), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on variants are not supported");
        }
    };
}

#[test]
fn deny_variant_field_attributes() {
    assert_errors! {
        r#"enum Enum { Variant { #[field_attribute] field } }"#,
        span!(22, 40), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on variant fields are not supported");
        }
    };
}

#[test]
fn deny_expr_attributes() {
    assert_errors! {
        r#"pub fn main() { #[expr_attribute] 42 }"#,
        span!(16, 33), Custom { error } => {
            assert_eq!(error.to_string(), "Attributes on expressions are not supported");
        }
    };
}

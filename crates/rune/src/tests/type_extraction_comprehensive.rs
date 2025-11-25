//! Comprehensive tests for type extraction API.

prelude!();

use crate::tests::compile_helper;

/// Basic function signature extraction
#[test]
fn extract_basic_signatures() {
    let mut sources = Sources::new();
    sources.insert(Source::memory(
        r#"
        pub fn add(a: i64, b: i64) -> i64 {
            a + b
        }
        "#
    )).unwrap();

    let mut diagnostics = crate::Diagnostics::new();
    let unit = compile_helper(&sources, &mut diagnostics).expect("should compile");

    // Verify we can extract type info
    let types = unit.extract_all_type_info();
    assert!(!types.is_empty(), "Should extract type information");
}

/// Tuple return type extraction
#[test]
fn extract_tuple_types() {
    let mut sources = Sources::new();
    sources.insert(Source::memory(
        r#"
        pub fn pair() -> (i64, String) {
            (42, "hello")
        }
        "#
    )).unwrap();

    let mut diagnostics = crate::Diagnostics::new();
    let unit = compile_helper(&sources, &mut diagnostics).expect("should compile");

    let types = unit.extract_all_type_info();
    assert!(!types.is_empty(), "Should extract tuple type info");
}

/// Struct type extraction
#[test]
fn extract_struct_types() {
    let mut sources = Sources::new();
    sources.insert(Source::memory(
        r#"
        struct Point {
            x: i64,
            y: i64,
        }

        pub fn create_point() -> Point {
            Point { x: 1, y: 2 }
        }
        "#
    )).unwrap();

    let mut diagnostics = crate::Diagnostics::new();
    let unit = compile_helper(&sources, &mut diagnostics).expect("should compile");

    let types = unit.extract_all_type_info();
    assert!(!types.is_empty(), "Should extract struct type info");
}

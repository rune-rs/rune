//! Tests for type extraction API.
//!
//! These tests verify that embedders can query type information from compiled units.
//! Run with: cargo test --features gradual-typing -p rune type_extraction

prelude!();

// ============================================================================
// Type Extraction API
// ============================================================================

/// Can extract function signature from compiled unit
#[test]
fn extract_function_signature() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        /// Adds two numbers together
        pub fn add(a: i64, b: i64) -> i64 {
            a + b
        }

        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    // Extract function signature
    let sig = unit
        .function_signature_by_name("add")
        .expect("should find 'add' function");

    assert_eq!(sig.name, "add");
    assert_eq!(sig.parameters.len(), 2);
    assert_eq!(sig.parameters[0].name, "a");
    assert_eq!(sig.parameters[1].name, "b");

    // Check parameter types
    let param_a_type = sig.parameters[0]
        .type_info
        .as_ref()
        .expect("param 'a' should have type");
    assert_eq!(param_a_type.to_type_string(), "i64");

    let param_b_type = sig.parameters[1]
        .type_info
        .as_ref()
        .expect("param 'b' should have type");
    assert_eq!(param_b_type.to_type_string(), "i64");

    // Check return type
    let return_type = sig.return_type.as_ref().expect("should have return type");
    assert_eq!(return_type.to_type_string(), "i64");
}

/// Can extract all function signatures from unit
#[test]
fn extract_all_signatures() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn foo(x: i64) -> i64 { x }
        pub fn bar(s: String) -> String { s }
        fn private_fn() {}
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let signatures: Vec<_> = unit.function_signatures().collect();

    // Should find foo, bar, main, and private_fn (all functions are included)
    let names: Vec<_> = signatures.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"foo"), "should contain 'foo'");
    assert!(names.contains(&"bar"), "should contain 'bar'");
    assert!(names.contains(&"main"), "should contain 'main'");
}

/// Untyped parameters report as None
#[test]
fn untyped_params_are_none() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn dynamic_fn(x, y) {
            x + y
        }
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let sig = unit
        .function_signature_by_name("dynamic_fn")
        .expect("should find 'dynamic_fn'");

    // Parameters should be None (dynamic/untyped)
    assert!(
        sig.parameters[0].type_info.is_none(),
        "untyped param should be None"
    );
    assert!(
        sig.parameters[1].type_info.is_none(),
        "untyped param should be None"
    );
    assert!(sig.return_type.is_none(), "untyped return should be None");
}

/// Can convert AnnotatedType to string representation
#[test]
fn type_info_to_string() {
    use crate::compile::type_info::AnnotatedType;

    // Named type
    let int_type = AnnotatedType::Named {
        path: "i64".try_into().unwrap(),
    };
    assert_eq!(int_type.to_type_string(), "i64");

    // Tuple type
    let tuple_type = AnnotatedType::Tuple(
        vec![
            AnnotatedType::Named {
                path: "i64".try_into().unwrap(),
            },
            AnnotatedType::Named {
                path: "String".try_into().unwrap(),
            },
        ]
        .try_into()
        .unwrap(),
    );
    assert_eq!(tuple_type.to_type_string(), "(i64, String)");

    // Never type
    assert_eq!(AnnotatedType::Never.to_type_string(), "!");
}

/// Can lookup function by hash
#[test]
fn lookup_by_hash() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn target(x: i64) -> i64 { x }
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    // Get hash for function - functions are at the root, so just "target"
    let hash = Hash::type_hash(["target"]);
    let sig = unit.function_signature(hash);

    assert!(sig.is_some(), "should find function by hash");
    assert_eq!(sig.unwrap().name, "target");
}

/// Tuple return types are extracted correctly
#[test]
fn tuple_return_type() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn get_pair() -> (i64, String) {
            (42, "hello")
        }
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let sig = unit
        .function_signature_by_name("get_pair")
        .expect("should find 'get_pair'");

    let return_type = sig.return_type.as_ref().expect("should have return type");
    assert_eq!(return_type.to_type_string(), "(i64, String)");
}

/// Mixed typed and untyped parameters in same function
#[test]
fn mixed_typed_untyped() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn mixed(a: i64, b, c: String) -> i64 {
            a
        }
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let sig = unit
        .function_signature_by_name("mixed")
        .expect("should find 'mixed'");

    assert_eq!(sig.parameters.len(), 3);

    // First param typed
    assert!(sig.parameters[0].type_info.is_some());
    assert_eq!(
        sig.parameters[0]
            .type_info
            .as_ref()
            .unwrap()
            .to_type_string(),
        "i64"
    );

    // Second param untyped
    assert!(sig.parameters[1].type_info.is_none());

    // Third param typed
    assert!(sig.parameters[2].type_info.is_some());
    assert_eq!(
        sig.parameters[2]
            .type_info
            .as_ref()
            .unwrap()
            .to_type_string(),
        "String"
    );
}

/// AnnotatedType is_primitive detection
#[test]
fn primitive_type_detection() {
    use crate::compile::type_info::AnnotatedType;

    let primitives = ["i64", "f64", "bool", "String", "char", "u8"];

    for name in primitives {
        let ty = AnnotatedType::Named {
            path: name.try_into().unwrap(),
        };
        assert!(ty.is_primitive(), "{name} should be primitive");
    }

    let non_primitive = AnnotatedType::Named {
        path: "MyStruct".try_into().unwrap(),
    };
    assert!(
        !non_primitive.is_primitive(),
        "MyStruct should not be primitive"
    );

    let tuple = AnnotatedType::Tuple(vec![].try_into().unwrap());
    assert!(!tuple.is_primitive(), "tuple should not be primitive");

    assert!(
        !AnnotatedType::Never.is_primitive(),
        "never should not be primitive"
    );
}

/// Path types (module::Type) are extracted correctly
#[test]
fn path_type_extraction() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        pub fn get_option() -> Option {
            Some(42)
        }
        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let sig = unit
        .function_signature_by_name("get_option")
        .expect("should find 'get_option'");

    let return_type = sig.return_type.as_ref().expect("should have return type");
    assert_eq!(return_type.to_type_string(), "Option");
}

// ============================================================================
// Struct Type Extraction API
// ============================================================================

/// Can extract struct info from compiled unit
#[test]
fn extract_struct_info() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        struct Person {
            name,
            age,
        }

        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    // Extract struct info
    let info = unit
        .struct_info_by_name("Person")
        .expect("should find 'Person' struct");

    assert_eq!(info.name.as_ref(), "Person");
    assert_eq!(info.fields.len(), 2);

    // Find name field
    let name_field = info.fields.iter().find(|f| f.name.as_ref() == "name");
    assert!(name_field.is_some(), "should have 'name' field");

    // Find age field
    let age_field = info.fields.iter().find(|f| f.name.as_ref() == "age");
    assert!(age_field.is_some(), "should have 'age' field");
}

/// Can extract all struct infos from unit
#[test]
fn extract_all_struct_infos() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        struct Point { x, y }
        struct Rect { top_left, bottom_right }

        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let structs: Vec<_> = unit.struct_infos().collect();

    // Should find Point and Rect
    let names: Vec<_> = structs.iter().map(|s| s.name.as_ref()).collect();
    assert!(names.contains(&"Point"), "should contain 'Point'");
    assert!(names.contains(&"Rect"), "should contain 'Rect'");
}

/// Can lookup struct by hash
#[test]
fn lookup_struct_by_hash() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        struct Target { field }

        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    // Get hash for struct
    let hash = Hash::type_hash(["Target"]);
    let info = unit.struct_info(hash);

    assert!(info.is_some(), "should find struct by hash");
    assert_eq!(info.unwrap().name.as_ref(), "Target");
}

/// Struct fields have correct positions
#[test]
fn struct_field_positions() {
    let context = Context::with_default_modules().unwrap();
    let mut sources = Sources::new();
    sources
        .insert(
            Source::new(
                "main",
                r#"
        struct Data {
            first,
            second,
            third,
        }

        pub fn main() {}
    "#,
            )
            .unwrap(),
        )
        .unwrap();

    let mut diagnostics = Diagnostics::new();
    let unit = crate::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build()
        .unwrap();

    let info = unit
        .struct_info_by_name("Data")
        .expect("should find 'Data' struct");

    // Fields should have sequential positions
    let first = info.fields.iter().find(|f| f.name.as_ref() == "first");
    let second = info.fields.iter().find(|f| f.name.as_ref() == "second");
    let third = info.fields.iter().find(|f| f.name.as_ref() == "third");

    assert!(first.is_some());
    assert!(second.is_some());
    assert!(third.is_some());
}

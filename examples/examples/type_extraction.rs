//! Example: Extracting type information from compiled Rune scripts
//!
//! This example demonstrates how to use the type extraction API to query
//! function signatures and struct definitions from compiled Rune code.
//!
//! Run with: `cargo run --example type_extraction`

use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, FunctionSignature, Hash, StructInfo};

fn main() -> rune::support::Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = rune::sources! {
        entry => {
            // Struct with typed fields
            struct Point {
                x: i64,
                y: i64,
            }

            // Struct with mixed typed/untyped fields
            struct Config {
                name: String,
                value,
            }

            pub fn add(a: i64, b: i64) -> i64 {
                a + b
            }

            pub fn greet(name: String) -> String {
                name
            }

            pub fn dynamic(x, y) {
                x + y
            }

            pub async fn fetch_data(url: String) -> String {
                url
            }

            pub fn create_point(x: i64, y: i64) -> Point {
                Point { x, y }
            }
        }
    };

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;

    // Iterate all function signatures
    println!("=== All Functions ===\n");
    for sig in unit.function_signatures() {
        print_signature(&sig);
        println!();
    }

    // Lookup by name
    println!("=== Lookup 'add' by name ===\n");
    if let Some(sig) = unit.function_signature_by_name("add") {
        print_signature(&sig);
    } else {
        println!("Function 'add' not found");
    }

    // Lookup by hash
    println!("\n=== Lookup 'greet' by hash ===\n");
    let hash = Hash::type_hash(["greet"]);
    if let Some(sig) = unit.function_signature(hash) {
        print_signature(&sig);
    } else {
        println!("Function 'greet' not found");
    }

    // === Struct Information ===
    // Note: Struct field type annotations require debug info support which
    // is not yet fully implemented. Field types will show as "dynamic" until
    // DebugStruct is populated during compilation.
    println!("\n=== All Structs ===\n");
    for info in unit.struct_infos() {
        print_struct_info(&info);
        println!();
    }

    // Lookup struct by name
    println!("=== Lookup 'Point' by name ===\n");
    if let Some(info) = unit.struct_info_by_name("Point") {
        print_struct_info(&info);
    } else {
        println!("Struct 'Point' not found");
    }

    Ok(())
}

fn print_signature(sig: &FunctionSignature) {
    println!("Function: {}", sig.name);
    println!("  Path: {}", sig.path);
    println!("  Hash: {}", sig.hash);
    println!("  Async: {}", sig.is_async);

    println!("  Parameters:");
    if sig.parameters.is_empty() {
        println!("    (none)");
    } else {
        for param in &sig.parameters {
            match &param.type_info {
                Some(t) => println!("    {}: {}", param.name, t.to_type_string()),
                None => println!("    {}: dynamic", param.name),
            }
        }
    }

    match &sig.return_type {
        Some(t) => println!("  Returns: {}", t.to_type_string()),
        None => println!("  Returns: dynamic"),
    }
}

fn print_struct_info(info: &StructInfo) {
    println!("Struct: {}", info.name);
    println!("  Path: {}", info.path);
    println!("  Hash: {}", info.hash);

    println!("  Fields:");
    if info.fields.is_empty() {
        println!("    (none)");
    } else {
        for field in info.fields.iter() {
            match &field.type_info {
                Some(t) => println!("    {}: {}", field.name, t.to_type_string()),
                None => println!("    {}: dynamic", field.name),
            }
        }
    }
}

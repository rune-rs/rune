//! Example: Extracting function signatures from compiled Rune scripts
//!
//! This example demonstrates how to use the type extraction API to query
//! function signatures including parameter names, types, and return types
//! from compiled Rune code.
//!
//! Run with: `cargo run --example type_extraction`

use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, FunctionSignature, Hash};

fn main() -> rune::support::Result<()> {
    let context = Context::with_default_modules()?;

    let mut sources = rune::sources! {
        entry => {
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

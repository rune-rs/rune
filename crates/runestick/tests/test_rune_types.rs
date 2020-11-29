/*!

Tests for `std::any::type_name_of_val(v)`
*/

use rune::{
    load_sources,
    termcolor::{ColorChoice, StandardStream},
    EmitDiagnostics as _, Errors, Options, Sources, Warnings,
};
use runestick::{FromValue, Source, Vm};
use std::sync::Arc;

fn make_vm(src: &str) -> Result<Vm, Box<dyn std::error::Error>> {
    let context = runestick::Context::with_default_modules()?;
    let context = Arc::new(context);

    let mut sources = Sources::new();
    sources.insert(Source::new("test.rn", src));

    let mut errors = Errors::new();
    let options = Options::default();
    let mut warnings = Warnings::new();

    let unit = match load_sources(&context, &options, &mut sources, &mut errors, &mut warnings) {
        Ok(e) => e,
        Err(_) => {
            let mut writer = StandardStream::stderr(ColorChoice::Always);
            errors
                .emit_diagnostics(&mut writer, &sources)
                .expect("failed writing errors");
            panic!("x");
        }
    };
    let unit = Arc::new(unit);

    let vm = Vm::new(Arc::new(context.runtime()), unit.clone());
    Ok(vm)
}

#[test]
fn test_trivial_types() {
    let vm = make_vm(r#"
use std::any::type_name_of_val;
pub fn main() {
(type_name_of_val(true), type_name_of_val(1), type_name_of_val(1.0), type_name_of_val('c'), type_name_of_val("s"), type_name_of_val(Some("s")))
}"#).expect("created vm");

    assert_eq!(
        <(String, String, String, String, String, String)>::from_value(
            vm.call(&["main"], ()).expect("failed call")
        )
        .expect("wrong type"),
        (
            "::std::core::bool".to_owned(),
            "::std::core::int".to_owned(),
            "::std::core::float".to_owned(),
            "::std::core::char".to_owned(),
            "::std::string::String".to_owned(),
            "::std::option::Option".to_owned()
        )
    )
}

#[test]
fn test_fn_types() {
    let vm = make_vm(
        r#"
use std::any::type_name_of_val;
fn foo() {}
mod bar { pub fn foo() {} }
pub fn main() {
(type_name_of_val(foo), type_name_of_val(bar::foo))
}"#,
    )
    .expect("created vm");

    assert_eq!(
        <(String, String)>::from_value(vm.call(&["main"], ()).expect("failed call"))
            .expect("wrong type"),
        ("foo".to_owned(), "bar::foo".to_owned(),)
    )
}

#[test]
fn test_struct() {
    let vm = make_vm(
        r#"
use std::any::type_name_of_val;

struct X{}
impl X{ 
    fn foo(self) {} 
    fn ctor() { X{} }
}
pub fn main() {
    let x = X{};
    (type_name_of_val(x), type_name_of_val(X::ctor), type_name_of_val(X::foo))
}"#,
    )
    .expect("created vm");

    assert_eq!(
        <(String, String, String)>::from_value(vm.call(&["main"], ()).expect("failed call"))
            .expect("wrong type"),
        ("X".to_owned(), "X::ctor".to_owned(), "X::foo".to_owned())
    )
}

#[test]
fn test_enum() {
    let vm = make_vm(
        r#"
use std::any::type_name_of_val;

enum E {
   A{ f },
   B(g),
   C,
}

pub fn main() {
    let ea = E::A { f: 1 };
    let eb = E::B(2);
    let ec = E::C;

    (type_name_of_val(ea), type_name_of_val(eb), type_name_of_val(ec))
}"#,
    )
    .expect("created vm");

    assert_eq!(
        <(String, String, String)>::from_value(vm.call(&["main"], ()).expect("failed call"))
            .expect("wrong type"),
        ("E".to_owned(), "E".to_owned(), "E".to_owned())
    )
}

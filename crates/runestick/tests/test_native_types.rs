//! Tests for `std::any::type_name_of_val(v)` for native types

use rune::{
    load_sources,
    termcolor::{ColorChoice, StandardStream},
    EmitDiagnostics as _, Errors, Options, Sources, Warnings,
};
use runestick::{FromValue, Module, Source, Vm};
use std::sync::Arc;

#[derive(runestick::Any)]
pub struct NativeStruct(pub u32);

pub fn native_fn() {}

impl NativeStruct {
    pub fn instance_fn(&self) {}
    pub fn get_x(&self) -> u32 {
        self.0
    }
}

fn make_vm(src: &str) -> Result<Vm, Box<dyn std::error::Error>> {
    let mut context = runestick::Context::with_default_modules()?;

    let mut module = Module::with_crate("native_crate");
    module.ty::<NativeStruct>()?;
    module.function(&["native_fn"], native_fn)?;
    module.inst_fn("instance_fn", NativeStruct::instance_fn)?;
    module.field_fn(runestick::Protocol::GET, "x", NativeStruct::get_x)?;

    context.install(&module).expect("installing module");

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
fn test_struct() {
    let vm =
        make_vm("pub fn main(v) { std::any::type_name_of_val(v) }").expect("failed creating VM");
    let t1 = NativeStruct(1);
    assert_eq!(
        String::from_value(vm.call(&["main"], (&t1,)).expect("failed call")).expect("wrong type"),
        "::native_crate::NativeStruct"
    );
}

#[test]
fn test_fn() {
    let vm = make_vm("pub fn main() { std::any::type_name_of_val(native_crate::native_fn) }")
        .expect("failed creating VM");

    assert_eq!(
        String::from_value(vm.call(&["main"], ()).expect("failed call")).expect("wrong type"),
        "::native_crate::native_fn"
    );
}

#[test]
fn test_inst_fn() {
    let vm = make_vm(
        "pub fn main() { std::any::type_name_of_val(native_crate::NativeStruct::instance_fn) }",
    )
    .expect("failed creating VM");

    assert_eq!(
        String::from_value(vm.call(&["main"], ()).expect("failed call")).expect("wrong type"),
        "::native_crate::NativeStruct::instance_fn"
    );
}

#[test]
fn test_field_fn() {
    let vm = make_vm("pub fn main(val) { std::any::type_name_of_val(val.x) }")
        .expect("failed creating VM");
    let t1 = NativeStruct(1);
    assert_eq!(
        String::from_value(vm.call(&["main"], (&t1,)).expect("failed call")).expect("wrong type"),
        "::std::core::int"
    );
}

// Not sure what the right return should be here - it returns the field name, but it probably should return ::std::core::int?
// #[test]
// fn test_field_fn_ref() {
//     let vm = make_vm("pub fn main() { std::any::type_name_of_val(native_crate::NativeStruct::x) }")
//         .expect("failed creating VM");
//     assert_eq!(
//         String::from_value(vm.call(&["main"], ()).expect("failed call")).expect("wrong type"),
//         "::std::core::int"
//     );
// }

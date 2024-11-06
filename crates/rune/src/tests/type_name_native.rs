//! Tests for `std::any::type_name_of_val(v)` for native types

prelude!();

#[derive(Any)]
#[rune(item = ::native_crate)]
pub struct NativeStruct(pub u32);

pub fn native_fn() {}

impl NativeStruct {
    pub fn instance_fn(&self) {}
    pub fn get_x(&self) -> u32 {
        self.0
    }
}

fn make_native_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("native_crate")?;
    module.ty::<NativeStruct>()?;
    module.function("native_fn", native_fn).build()?;
    module.associated_function("instance_fn", NativeStruct::instance_fn)?;
    module.field_function(&Protocol::GET, "x", NativeStruct::get_x)?;

    Ok(module)
}

#[test]
fn test_struct() {
    let t1 = NativeStruct(1);

    let m = make_native_module().expect("failed making native module");

    let s: String = rune_n! {
        mod m,
        (t1,),
        pub fn main(v) { std::any::type_name_of_val(v) }
    };

    assert_eq!(s, "::native_crate::NativeStruct");
}

#[test]
fn test_fn() {
    let m = make_native_module().expect("failed making native module");

    let s: String = rune_n! {
        mod m,
        (),
        pub fn main() { std::any::type_name_of_val(native_crate::native_fn) }
    };

    assert_eq!(s, "::std::ops::Function");
}

#[test]
fn test_inst_fn() {
    let m = make_native_module().expect("failed making native module");

    let s: String = rune_n! {
        mod m,
        (),
        pub fn main() {
            std::any::type_name_of_val(native_crate::NativeStruct::instance_fn)
        }
    };

    assert_eq!(s, "::std::ops::Function");
}

#[test]
fn test_field_fn() {
    let t1 = NativeStruct(1);

    let m = make_native_module().expect("failed making native module");

    let s: String = rune_n! {
        mod m,
        (t1, ),
        pub fn main(val) { std::any::type_name_of_val(val.x) }
    };

    assert_eq!(s, "::std::u64");
}

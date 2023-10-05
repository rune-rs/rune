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
    module.field_function(Protocol::GET, "x", NativeStruct::get_x)?;

    Ok(module)
}

#[test]
fn test_struct() {
    let t1 = NativeStruct(1);
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            String =>
                pub fn main(v) { std::any::type_name_of_val(v) }
        },
        "::native_crate::NativeStruct"
    );
}

#[test]
fn test_fn() {
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (),
            String => pub fn main() { std::any::type_name_of_val(native_crate::native_fn) }
        },
        "::std::ops::Function"
    );
}

#[test]
fn test_inst_fn() {
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (),
            String =>
            pub fn main() {
                std::any::type_name_of_val(native_crate::NativeStruct::instance_fn)
            }
        },
        "::std::ops::Function"
    );
}

#[test]
fn test_field_fn() {
    let t1 = NativeStruct(1);
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            String => pub fn main(val) { std::any::type_name_of_val(val.x) }
        },
        "::std::i64"
    );
}

// Not sure what the right return should be here - it returns the field name, but it probably should return ::std::i64?
// #[test]
// fn test_field_fn_ref() {
//     assert_eq!(
//         rune_n! {
//             make_native_module().expect("failed making native module"),
//             (),
//             String =>
//             pub fn main() {
//                 std::any::type_name_of_val(native_crate::NativeStruct::x)
//             }
//         },
//         "::std::i64"
//     );
// }

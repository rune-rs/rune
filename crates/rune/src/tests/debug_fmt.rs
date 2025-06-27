//! Tests for `std::any::type_name_of_val(v)` for native types

prelude!();

use rune::alloc;

#[derive(Any, Debug)]
#[rune(item = ::native_crate)]
pub struct NativeStructWithProtocol;

impl NativeStructWithProtocol {
    #[rune::function(protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> alloc::Result<()> {
        write!(f, "{self:?}")
    }
}

#[derive(Any)]
#[rune(item = ::native_crate)]
pub struct NativeStructWithoutProtocol;

fn make_native_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("native_crate")?;
    module.ty::<NativeStructWithProtocol>()?;
    module.function_meta(NativeStructWithProtocol::debug_fmt)?;
    module.ty::<NativeStructWithoutProtocol>()?;
    Ok(module)
}

#[test]
fn test_with_debug_fmt() {
    let t1 = NativeStructWithProtocol;

    let m = make_native_module().unwrap();

    let s: String = rune_n! {
        mod m,
        (t1,),
        pub fn main(v) { format!("{v:?}") }
    };

    assert_eq!(s, "NativeStructWithProtocol");
}

#[test]
fn test_without_debug_fmt() {
    let t1 = NativeStructWithoutProtocol;

    let m = make_native_module().unwrap();

    let result: String = rune_n! {
        mod m,
        (t1,),
        pub fn main(v) { format!("{v:?}") }
    };

    assert!(
        result.starts_with("<::native_crate::NativeStructWithoutProtocol object at 0x"),
        "Expected '<::native_crate::NativeStructWithoutProtocol object at 0x', got: {result:?}",
    );
}

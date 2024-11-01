//! Tests for `std::any::type_name_of_val(v)` for native types

prelude!();

#[derive(Any, Debug)]
#[rune(item = ::native_crate)]
pub struct NativeStructWithProtocol;

impl NativeStructWithProtocol {
    #[rune::function(protocol = DEBUG_FMT)]
    fn debug_fmt(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "{self:?}")
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
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            String =>
            pub fn main(v) { format!("{:?}", v) }
        },
        "NativeStructWithProtocol"
    );
}

#[test]
fn test_without_debug_fmt() {
    let t1 = NativeStructWithoutProtocol;
    let result = rune_n! {
        make_native_module().expect("failed making native module"),
        (t1, ),
        String =>
            pub fn main(v) { format!("{:?}", v) }
    };

    assert!(
        result.starts_with("<::native_crate::NativeStructWithoutProtocol object at 0x"),
        "Expected '<::native_crate::NativeStructWithoutProtocol object at 0x', got: {:?}",
        result
    );
}

prelude!();

#[derive(Default)]
struct MyAny {}

crate::__internal_impl_any!(self, MyAny);

fn get_vm() -> crate::support::Result<crate::Vm> {
    use std::sync::Arc;

    let mut sources = crate::sources! {
        entry => {
            enum Enum {
                Variant(internal)
            }

            struct Struct(internal);

            pub fn function(argument) {}
        }
    };

    let context = crate::Context::with_default_modules()?;
    let unit = crate::prepare(&mut sources).build()?;
    Ok(crate::Vm::new(Arc::new(context.runtime()?), Arc::new(unit)))
}

#[test]
fn references_allowed_for_function_calls() {
    let vm = get_vm().unwrap();
    let function = vm.lookup_function(["function"]).unwrap();

    let value_result = function.call::<crate::Value>((crate::Value::unit(),));
    assert!(value_result.is_ok());

    let mut mine = MyAny::default();

    let ref_result = function.call::<crate::Value>((&mine,));
    assert!(ref_result.is_ok());

    let mut_result = function.call::<crate::Value>((&mut mine,));
    assert!(mut_result.is_ok());

    let any_result = function.call::<crate::Value>((mine,));
    assert!(any_result.is_ok());
}

#[test]
fn references_disallowed_for_tuple_variant() {
    use crate::runtime::{VmErrorKind, VmResult};

    let vm = get_vm().unwrap();
    let constructor = vm.lookup_function(["Enum", "Variant"]).unwrap();

    let value_result = constructor.call::<crate::Value>((crate::Value::unit(),));
    assert!(value_result.is_ok());

    let mut mine = MyAny::default();

    let VmResult::Err(ref_error) = constructor.call::<crate::Value>((&mine,)) else {
        panic!("expected ref call to return an error")
    };
    assert_eq!(ref_error.into_kind(), VmErrorKind::InvalidTupleCall);

    let VmResult::Err(mut_error) = constructor.call::<crate::Value>((&mut mine,)) else {
        panic!("expected mut call to return an error")
    };
    assert_eq!(mut_error.into_kind(), VmErrorKind::InvalidTupleCall);

    let any_result = constructor.call::<crate::Value>((mine,));
    assert!(any_result.is_ok());
}

#[test]
fn references_disallowed_for_tuple_struct() {
    use crate::runtime::{VmErrorKind, VmResult};

    let vm = get_vm().unwrap();
    let constructor = vm.lookup_function(["Struct"]).unwrap();

    let value_result = constructor.call::<crate::Value>((crate::Value::unit(),));
    assert!(value_result.is_ok());

    let mut mine = MyAny::default();

    let VmResult::Err(ref_error) = constructor.call::<crate::Value>((&mine,)) else {
        panic!("expected ref call to return an error")
    };
    assert_eq!(ref_error.into_kind(), VmErrorKind::InvalidTupleCall);

    let VmResult::Err(mut_error) = constructor.call::<crate::Value>((&mut mine,)) else {
        panic!("expected mut call to return an error")
    };
    assert_eq!(mut_error.into_kind(), VmErrorKind::InvalidTupleCall);

    let any_result = constructor.call::<crate::Value>((mine,));
    assert!(any_result.is_ok());
}
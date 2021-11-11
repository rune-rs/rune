//! Tests for derive(Any) on generic types

use rune::{Any, ContextError, Module, Named, ToValue, UnsafeFromValue};
use rune_tests::*;

#[derive(Any)]
struct Generic<T>
where
    T: 'static + Clone + Named + UnsafeFromValue + ToValue,
{
    #[rune(get, set)]
    data: T,
}

impl<T> Generic<T>
where
    T: 'static + Clone + Copy + Named + UnsafeFromValue + ToValue,
{
    fn get_value(&self) -> T {
        self.data
    }
}

fn make_native_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("native_crate");
    module.ty::<Generic<i64>>()?;
    module.inst_fn("get_value", Generic::<i64>::get_value)?;

    module.ty::<Generic<f64>>()?;
    module.inst_fn("get_value", Generic::<f64>::get_value)?;
    Ok(module)
}

#[test]
fn test_generic_i64() {
    let t1 = Generic { data: 1i64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            i64 =>
                pub fn main(v) { v.get_value() }
        },
        1
    );
}

#[test]
fn test_generic_i64_get() {
    let t1 = Generic { data: 2i64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            i64 =>
                pub fn main(v) { v.data }
        },
        2
    );
}

#[test]
fn test_generic_i64_set() {
    let t1 = Generic { data: 2i64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            ( t1, ),
            i64 =>
                pub fn main(v) { v.data = 10; v.data }
        },
        10
    );
}

#[test]
fn test_generic_f64() {
    let t1 = Generic { data: 2f64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            f64 =>
                pub fn main(v) { v.get_value() }
        },
        2.0
    );
}

#[test]
fn test_generic_f64_get() {
    let t1 = Generic { data: 2f64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            (t1, ),
            f64 =>
                pub fn main(v) { v.data }
        },
        2.0
    );
}

#[test]
fn test_generic_f64_set() {
    let t1 = Generic { data: 2f64 };
    assert_eq!(
        rune_n! {
            make_native_module().expect("failed making native module"),
            ( t1, ),
            f64 =>
                pub fn main(v) { v.data = 10.0; v.data }
        },
        10.0
    );
}

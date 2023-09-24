//! Tests for derive(Any) on generic types

prelude!();

use std::sync::Arc;

use anyhow::{anyhow, bail, Context as _, Result};

use rune::compile::Named;
use rune::runtime::{MaybeTypeOf, ToValue, Type, TypeOf};
use rune::termcolor;
use rune::{Any, Context, ContextError, Diagnostics, Module, Sources, Vm};

#[derive(Any)]
#[rune(item = ::native_crate)]
struct Generic<T>
where
    T: 'static + Clone + Named + FromValue + ToValue + MaybeTypeOf + TypeOf,
{
    #[rune(get, set)]
    data: T,
}

impl<T> Generic<T>
where
    T: 'static + Clone + Copy + Named + FromValue + ToValue + MaybeTypeOf + TypeOf,
{
    fn get_value(&self) -> T {
        self.data
    }
}

fn make_native_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate("native_crate")?;
    module.ty::<Generic<i64>>()?;
    module.associated_function("get_value", Generic::<i64>::get_value)?;

    module.ty::<Generic<f64>>()?;
    module.associated_function("get_value", Generic::<f64>::get_value)?;
    Ok(module)
}

fn compile(mut sources: Sources) -> Result<Vm> {
    let mut context = Context::with_default_modules()?;
    context.install(make_native_module()?)?;
    let mut diagnostics = Diagnostics::default();
    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut out = termcolor::Buffer::no_color();
        diagnostics.emit(&mut out, &sources)?;
        let out = String::from_utf8(out.into_inner())?;
        bail!("{out}")
    }

    let unit = result?;
    Ok(Vm::new(Arc::new(context.runtime()?), Arc::new(unit)))
}

// This is similar to the generic test that existed before, but ensures that the
// generic type can exist in multiple instances in the same Rune script.
#[test]
fn test_generic() -> Result<()> {
    macro_rules! test {
        ($($ty:ty, $function:ident, $function_ty:ident, $value:expr, $set:expr),+ $(,)?) => {
            let mut vm = compile(rune::sources! {
                entry => {
                    pub fn get_value(v) { v.get_value() }
                    pub fn get_data(v) { v.data }
                    pub fn set_data(v, value) { v.data = value; }
                    $(pub fn $function(v) { ::native_crate::Generic::<$ty>::get_value(v) })*
                    $(pub fn $function_ty() { ::native_crate::Generic::<$ty> })*
                }
            })?;

            $(
                let t1: Generic<$ty> = Generic { data: $value };
                let value = vm.call(["get_value"], (t1,)).with_context(|| anyhow!("{}: get_value: Working call", stringify!($ty)))?;
                let value: $ty = rune::from_value(value).with_context(|| anyhow!("{}: get_value: Output value", stringify!($ty)))?;
                assert_eq!(value, $value);
            )*

            $(
                let t1: Generic<$ty> = Generic { data: $value };
                let value = vm.call(["get_data"], (t1,)).with_context(|| anyhow!("{}: get_data: Working call", stringify!($ty)))?;
                let value: $ty = rune::from_value(value).with_context(|| anyhow!("{}: get_data: Output value", stringify!($ty)))?;
                assert_eq!(value, $value);
            )*

            $(
                let mut t1: Generic<$ty> = Generic { data: $value };
                let _ = vm.call(["set_data"], (&mut t1, $set)).with_context(|| anyhow!("{}: set_data: Working call", stringify!($ty)))?;
                assert_eq!(t1.data, $set);
            )*

            $(
                let t1: Generic<$ty> = Generic { data: $value };
                let value = vm.call([stringify!($function)], (t1,)).with_context(|| anyhow!("{}: {}: Working call", stringify!($ty), stringify!($function)))?;
                let value: $ty = rune::from_value(value).with_context(|| anyhow!("{}: {}: Output value", stringify!($ty), stringify!($function)))?;
                assert_eq!(value, $value);
            )*

            $(
                let value = vm.call([stringify!($function_ty)], ()).with_context(|| anyhow!("{}: {}: Working call", stringify!($ty), stringify!($function_ty)))?;
                let value: Type = rune::from_value(value).with_context(|| anyhow!("{}: {}: Output value", stringify!($ty), stringify!($function_ty)))?;
                assert_eq!(<Generic::<$ty> as Any>::type_hash(), value.into_hash());
            )*
        };
    }

    test! {
        i64, test_int, test_int_ty, 3, 30,
        f64, test_float, test_float_ty, 2.0, 20.0,
    };

    Ok(())
}

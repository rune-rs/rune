//! Dynamic typing and type reflection.

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::{self, String};
use crate::runtime::{Formatter, Type, Value, VmError};
use crate::{docstring, ContextError, Hash, Module};

/// Dynamic typing and type reflection.
///
/// # `Type`
///
/// Values of this type indicates the type of any dynamic value and can be
/// constructed through the [`Type::of_val`] function.
#[rune::module(::std::any)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

    m.ty::<Type>()?.docs(docstring! {
        /// Represents a type in the Rune type system.
    })?;

    m.ty::<Hash>()?.docs(docstring! {
        /// Represents an opaque hash in the Rune type system.
    })?;

    m.function_meta(type_of_val)?;
    m.function_meta(type_name_of_val)?;
    m.function_meta(format_type)?;
    Ok(m)
}

/// Convert a value into a [`Type`] object.
///
/// # Examples
///
/// ```rune
/// let value1 = 42;
/// let value2 = 43;
/// let ty1 = Type::of_val(value1);
/// let ty2 = Type::of_val(value2);
/// assert_eq!(ty1, ty2);
/// ```
#[rune::function(free, path = Type::of_val)]
#[inline]
fn type_of_val(value: Value) -> Type {
    Type::new(value.type_hash())
}

/// Formatting a type.
///
/// # Examples
///
/// ```rune
/// use std::any;
///
/// assert_eq!(format!("{}", any::Type::of_val(42)), "Type(0x1cad9186c9641c4f)");
/// ```
#[rune::function(instance, protocol = DISPLAY_FMT)]
fn format_type(ty: Type, f: &mut Formatter) -> alloc::Result<()> {
    write!(f, "{ty:?}")
}

/// Get the type name of a value.
///
/// # Examples
///
/// ```rune
/// use std::any;
///
/// let value = 42;
/// assert_eq!(any::type_name_of_val(value), "::std::i64");
///
/// let value = [];
/// assert_eq!(any::type_name_of_val(value), "::std::vec::Vec");
/// ```
#[rune::function]
#[inline]
pub fn type_name_of_val(value: Value) -> Result<String, VmError> {
    value.into_type_name()
}

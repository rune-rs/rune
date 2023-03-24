#![allow(clippy::transmute_int_to_char)]

use std::mem;
use std::ptr;

use crate::{Hash, VmError};

/// A value in a virtual machine.
#[repr(C)]
pub struct Value {
    repr: [u8; 16],
}

test_size!(Value, rune::Value);

/// Construct a unit value.
///
/// Even though not strictly necessary, it is good practice to always free your
/// values with [rune_value_free].
///
/// \code{.c}
/// int main() {
///     rune_vm_value value = rune_value_unit();
///
///     // ...
///
///     rune_value_free(&value);
/// }
/// \endcode
#[no_mangle]
pub extern "C" fn rune_value_unit() -> Value {
    // Safety: this allocation is safe.
    unsafe { mem::transmute(rune::Value::Unit) }
}

/// Get the type hash of a value. Getting the type hash might error in case the
/// value is no longer accessible. If this happens, the empty hash is returned
/// and `error` is populated with the error that occured.
///
/// # Safety
///
/// The `value` argument must have been allocated with a function such as
/// [rune_value_unit] and a valid `error`.
#[no_mangle]
pub unsafe extern "C" fn rune_value_type_hash(
    value: *const Value,
    output: *mut Hash,
    error: *mut VmError,
) -> bool {
    let value = &*(value as *const rune::Value);

    let hash = match value.type_hash() {
        Ok(hash) => hash,
        Err(e) => {
            if error.is_null() {
                ptr::write(error, mem::transmute(e));
            } else {
                let _ = ptr::replace(error, mem::transmute(e));
            }

            return false;
        }
    };

    // Safety: the Hash type is Copy.
    ptr::write(output as *mut rune::Hash, hash);
    true
}

/// Simplified accessor for the type hash of the value which returns an
/// [rune_hash_empty][crate::rune_hash_empty] in case the type hash couldn't be
/// accessed and ignores the error.
///
/// # Safety
///
/// The `value` argument must have been allocated with a function such as
/// [rune_value_unit]`.
#[no_mangle]
pub unsafe extern "C" fn rune_value_type_hash_or_empty(value: *const Value) -> Hash {
    let value = &*(value as *const rune::Value);
    let hash = value.type_hash().unwrap_or(rune::Hash::EMPTY);
    mem::transmute(hash)
}

macro_rules! rune_value_init {
    ($name:ident, $variant:ident, $ty:ty) => {
        rune_value_init!($name, $variant, $ty, Into::into);
    };

    ($name:ident, $variant:ident, $ty:ty, $fn:path) => {
        /// Construct a value of the given type.
        #[no_mangle]
        pub extern "C" fn $name(value: $ty) -> Value {
            // Safety: this allocation is safe.
            unsafe { mem::transmute(rune::Value::$variant($fn(value))) }
        }
    };
}

rune_value_init!(rune_value_bool, Bool, bool);
rune_value_init!(rune_value_byte, Byte, u8);
rune_value_init!(rune_value_integer, Integer, i64);
rune_value_init!(rune_value_float, Float, f64);
rune_value_init!(rune_value_type, Type, Hash, mem::transmute);

/// Construct a character value.
///
/// Characters are only valid within the ranges smaller than 0x10ffff and not
/// within 0xD800 to 0xDFFF (inclusive).
///
/// If the pushed value is *not* within a valid character range, this function
/// returns `false`.
///
/// # Safety
///
/// The caller must ensure that `output` is allocated using something like
/// [rune_value_unit].
#[no_mangle]
pub unsafe extern "C" fn rune_value_char(value: u32, output: *mut Value) -> bool {
    if let Ok(value) = char::try_from(value) {
        let _ = ptr::replace(output as *mut rune::Value, rune::Value::Char(value));
        true
    } else {
        false
    }
}

/// Free the given value.
///
/// Strictly speaking, values which are Copy do not need to be freed, but you
/// should make a habit of freeing any value used anywhere.
///
/// This function is a little bit smart and sets the value to `Value::Unit` in
/// order to free it. This mitigates that subsequent calls to `rn_value_free`
/// doubly frees any allocated data.
///
/// # Safety
///
/// The `value` argument must have been allocated with a function such as
/// [rune_value_unit].
#[no_mangle]
pub unsafe extern "C" fn rune_value_free(value: *mut Value) {
    let _ = ptr::replace(value as *mut rune::Value, rune::Value::Unit);
}

/// Generate rune_value_set_* function for a copy type.
macro_rules! rune_value_set {
    ($name:ident, $variant:ident, $ty:ty) => {
        rune_value_set!($name, $variant, $ty, Into::into);
    };

    ($name:ident, $variant:ident, $ty:ty, $fn:path) => {
        /// Set the value to the given type.
        ///
        /// # Safety
        ///
        /// The `value` argument must have been allocated with a function such
        /// as [rune_value_unit].
        #[no_mangle]
        pub unsafe extern "C" fn $name(value: *mut Value, input: $ty) {
            let _ = ptr::replace(value as *mut rune::Value, rune::Value::$variant($fn(input)));
        }
    };
}

rune_value_set!(rune_value_set_bool, Bool, bool);
rune_value_set!(rune_value_set_byte, Byte, u8);
rune_value_set!(rune_value_set_char, Char, u32, mem::transmute);
rune_value_set!(rune_value_set_integer, Integer, i64);
rune_value_set!(rune_value_set_float, Float, f64);
rune_value_set!(rune_value_set_type, Type, Hash, mem::transmute);

/// Generate rune_value_is_* function for a copy type.
macro_rules! rune_value_is {
    ($name:ident, $($pat:pat_param)|* $(|)?) => {
        /// Test if the value is of the given type.
        ///
        /// # Safety
        ///
        /// The `value` argument must have been allocated with a function such
        /// as [rune_value_unit].
        #[no_mangle]
        pub unsafe extern "C" fn $name(value: *const Value) -> bool {
            matches!(&*(value as *mut rune::Value), $($pat)|*)
        }
    };
}

rune_value_is!(rune_value_is_unit, rune::Value::Unit);
rune_value_is!(rune_value_is_bool, rune::Value::Bool(..));
rune_value_is!(rune_value_is_byte, rune::Value::Byte(..));
rune_value_is!(rune_value_is_char, rune::Value::Char(..));
rune_value_is!(rune_value_is_integer, rune::Value::Integer(..));
rune_value_is!(rune_value_is_float, rune::Value::Float(..));
rune_value_is!(rune_value_is_type, rune::Value::Type(..));
rune_value_is!(
    rune_value_is_string,
    rune::Value::StaticString(..) | rune::Value::String(..)
);
rune_value_is!(rune_value_is_bytes, rune::Value::Bytes(..));
rune_value_is!(rune_value_is_vec, rune::Value::Vec(..));
rune_value_is!(rune_value_is_tuple, rune::Value::Tuple(..));
rune_value_is!(rune_value_is_object, rune::Value::Object(..));
rune_value_is!(rune_value_is_range, rune::Value::Range(..));
rune_value_is!(rune_value_is_future, rune::Value::Future(..));
rune_value_is!(rune_value_is_stream, rune::Value::Stream(..));
rune_value_is!(rune_value_is_generator, rune::Value::Generator(..));
rune_value_is!(
    rune_value_is_generatorstate,
    rune::Value::GeneratorState(..)
);
rune_value_is!(rune_value_is_option, rune::Value::Option(..));
rune_value_is!(rune_value_is_result, rune::Value::Result(..));
rune_value_is!(rune_value_is_unitstruct, rune::Value::UnitStruct(..));
rune_value_is!(rune_value_is_tuplestruct, rune::Value::TupleStruct(..));
rune_value_is!(rune_value_is_struct, rune::Value::Struct(..));
rune_value_is!(rune_value_is_variant, rune::Value::Variant(..));
rune_value_is!(rune_value_is_function, rune::Value::Function(..));
rune_value_is!(rune_value_is_format, rune::Value::Format(..));
rune_value_is!(rune_value_is_iterator, rune::Value::Iterator(..));
rune_value_is!(rune_value_is_any, rune::Value::Any(..));

/// Generate rune_value_as_* function for a copy type.
macro_rules! rune_value_as {
    ($name:ident, $variant:ident, $ty:ident) => {
        rune_value_as!($name, $variant, $ty, Into::into);
    };

    ($name:ident, $variant:ident, $ty:ident, $fn:path) => {
        /// Coerce value into the given type. If the coercion was successful
        /// returns `true` and consumed the value.
        ///
        /// # Safety
        ///
        /// The `value` argument must have been allocated with a function such
        /// as [rune_value_unit].
        #[no_mangle]
        pub unsafe extern "C" fn $name(value: *const Value, output: *mut $ty) -> bool {
            if let rune::Value::$variant(v) = &*(value as *mut rune::Value) {
                *output = $fn(*v);
                true
            } else {
                false
            }
        }
    };
}

rune_value_as!(rune_value_as_bool, Bool, bool);
rune_value_as!(rune_value_as_byte, Byte, u8);
rune_value_as!(rune_value_as_char, Char, char);
rune_value_as!(rune_value_as_integer, Integer, i64);
rune_value_as!(rune_value_as_float, Float, f64);
rune_value_as!(rune_value_as_type, Type, Hash, mem::transmute);

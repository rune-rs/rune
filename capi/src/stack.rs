use std::{mem, ptr};

use crate::{Hash, InternalVmError, Value, VmError};

/// The internal representation of a stack.
pub(crate) type StackInternal = rune::runtime::Stack;

/// The stack of a virtual machine.
#[repr(C)]
pub struct Stack {
    repr: [u8; 32],
}

test_size!(Stack, StackInternal);

/// Push a value onto the stack.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
#[no_mangle]
pub unsafe extern "C" fn rune_stack_push(stack: *mut Stack, value: Value) {
    let stack = &mut *(stack as *mut StackInternal);
    stack.push(mem::transmute::<_, rune::Value>(value));
}

/// Push a unit value onto the stack.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
#[no_mangle]
pub unsafe extern "C" fn rune_stack_push_unit(stack: *mut Stack) {
    let stack = &mut *(stack as *mut StackInternal);
    stack.push(rune::Value::Unit);
}

macro_rules! push {
    ($name:ident, $variant:ident, $ty:ty) => {
        push!($name, $variant, $ty, Into::into);
    };

    ($name:ident, $variant:ident, $ty:ty, $fn:path) => {
        /// Push a value with the given type onto the stack.
        ///
        /// # Safety
        ///
        /// Must be called with a valid stack. Like one fetched from
        /// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
        #[no_mangle]
        pub unsafe extern "C" fn $name(stack: *mut Stack, value: $ty) {
            let stack = &mut *(stack as *mut StackInternal);
            stack.push(rune::Value::$variant($fn(value)));
        }
    };
}

push!(rune_stack_push_bool, Bool, bool);
push!(rune_stack_push_byte, Byte, u8);
push!(rune_stack_push_integer, Integer, i64);
push!(rune_stack_push_float, Float, f64);
push!(rune_stack_push_type, Type, Hash, mem::transmute);

/// Push a character onto the stack. This variation pushes characters.
///
/// Characters are only valid within the ranges smaller than 0x10ffff and not
/// within 0xD800 to 0xDFFF (inclusive).
///
/// If the pushed value is *not* within a valid character range, this function
/// returns `false` and nothing will be pushed onto the stack.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
#[no_mangle]
pub unsafe extern "C" fn rune_stack_push_char(stack: *mut Stack, value: u32) -> bool {
    let stack = &mut *(stack as *mut StackInternal);

    if let Ok(value) = char::try_from(value) {
        stack.push(rune::Value::Char(value));
        true
    } else {
        false
    }
}

/// Push a tuple with `count` elements onto the stack. The components of the
/// tuple will be popped from the stack in the reverse order that they were
/// pushed.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
#[no_mangle]
pub unsafe extern "C" fn rune_stack_push_tuple(
    stack: *mut Stack,
    count: usize,
    error: *mut VmError,
) -> bool {
    let stack = &mut *(stack as *mut StackInternal);

    let it = match stack.drain(count) {
        Ok(it) => it,
        Err(e) => {
            let _ = ptr::replace(error as *mut InternalVmError, Some(e.into()));
            return false;
        }
    };

    let tuple = rune::runtime::Tuple::from_iter(it);
    stack.push(rune::Value::Tuple(rune::runtime::Shared::new(tuple)));
    true
}

/// Push a vector with `count` elements onto the stack. The elements of the
/// vector will be popped from the stack in the reverse order that they were
/// pushed.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut].
#[no_mangle]
pub unsafe extern "C" fn rune_stack_push_vec(
    stack: *mut Stack,
    count: usize,
    error: *mut VmError,
) -> bool {
    let stack = &mut *(stack as *mut StackInternal);

    let it = match stack.drain(count) {
        Ok(it) => it,
        Err(e) => {
            let _ = ptr::replace(error as *mut InternalVmError, Some(e.into()));
            return false;
        }
    };

    let vec = rune::runtime::Vec::from_iter(it);
    stack.push(rune::Value::Vec(rune::runtime::Shared::new(vec)));
    true
}

/// Pop an integer from the stack.
///
/// Return a boolean indicating if a value was popped off the stack. The value
/// is only populated if the popped value matched the given value.
///
/// # Safety
///
/// Must be called with a valid stack. Like one fetched from
/// [rune_vm_stack_mut][crate:rune_vm_stack_mut]. The `value` must also have
/// been allocated correctly.
#[no_mangle]
pub unsafe extern "C" fn rune_stack_pop_value(
    stack: *mut Stack,
    value: *mut Value,
    error: *mut VmError,
) -> bool {
    let stack = &mut *(stack as *mut StackInternal);

    let v = match stack.pop() {
        Ok(v) => v,
        Err(e) => {
            let _ = ptr::replace(error as *mut InternalVmError, Some(e.into()));
            return false;
        }
    };

    let _ = ptr::replace(value as *mut rune::Value, v);
    true
}

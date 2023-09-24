use core::mem;

use crate::ptr;

/// This replaces the value behind the `v` unique reference by calling the
/// relevant function.
///
/// If a panic occurs in the `change` closure, the entire process will be aborted.
#[allow(dead_code)] // keep as illustration and for future use
#[inline]
pub(crate) fn take_mut<T, E>(v: &mut T, change: impl FnOnce(T) -> Result<T, E>) -> Result<(), E> {
    replace(v, |value| Ok((change(value)?, ())))
}

/// This replaces the value behind the `v` unique reference by calling the
/// relevant function, and returns a result obtained along the way.
///
/// If a panic occurs in the `change` closure, the entire process will be aborted.
#[inline]
pub(crate) fn replace<T, R, E>(
    v: &mut T,
    change: impl FnOnce(T) -> Result<(T, R), E>,
) -> Result<R, E> {
    struct PanicGuard;

    impl Drop for PanicGuard {
        fn drop(&mut self) {
            crate::abort()
        }
    }

    let guard = PanicGuard;
    let value = unsafe { ptr::read(v) };
    let (new_value, ret) = change(value)?;
    unsafe {
        ptr::write(v, new_value);
    }
    mem::forget(guard);
    Ok(ret)
}

use crate::value::Slot;
use crate::vm::StackError;
use std::cell::{Cell, UnsafeCell};
use std::fmt;
use std::mem;
use std::ops;

#[derive(Debug, Clone, Default)]
pub(super) struct Access(Cell<isize>);

impl Access {
    /// Clear the given access token.
    #[inline]
    pub(super) fn clear(&self) {
        self.0.set(0);
    }

    /// Test if we have shared access without modifying the internal count.
    #[inline]
    pub(super) fn test_shared(&self, slot: Slot) -> Result<(), StackError> {
        let b = self.0.get().wrapping_sub(1);

        if b >= 0 {
            return Err(StackError::SlotInaccessibleShared { slot });
        }

        Ok(())
    }

    /// Mark that we want shared access to the given access token.
    #[inline]
    pub(super) fn shared(&self, slot: Slot) -> Result<(), StackError> {
        let b = self.0.get().wrapping_sub(1);

        if b >= 0 {
            return Err(StackError::SlotInaccessibleShared { slot });
        }

        self.0.set(b);
        Ok(())
    }

    /// Unshare the current access.
    #[inline]
    pub(super) fn release_shared(&self) {
        let b = self.0.get().wrapping_add(1);
        debug_assert!(b <= 0);
        self.0.set(b);
    }

    /// Unshare the current access.
    #[inline]
    pub(super) fn release_exclusive(&self) {
        let b = self.0.get().wrapping_sub(1);
        debug_assert!(b == 0);
        self.0.set(b);
    }

    /// Mark that we want exclusive access to the given access token.
    #[inline]
    pub(super) fn exclusive(&self, slot: Slot) -> Result<(), StackError> {
        let b = self.0.get().wrapping_add(1);

        if b != 1 {
            return Err(StackError::SlotInaccessibleExclusive { slot });
        }

        self.0.set(b);
        Ok(())
    }
}

/// Guard for a value borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
///
/// These also aid in function call integration, since they can be "arm" the
/// virtual machine to release shared guards through its unsafe functions.
///
/// See [disarm][Vm::disarm] for more information.
pub struct Ref<'a, T: ?Sized + 'a> {
    pub(super) value: &'a T,
    pub(super) access: &'a Access,
    pub(super) slot: Slot,
    pub(super) guards: &'a UnsafeCell<Vec<Slot>>,
}

impl<'a, T: ?Sized> Ref<'a, T> {
    /// Convert into a reference with an unbounded lifetime.
    ///
    /// # Safety
    ///
    /// The returned reference must not outlive the VM that produced it.
    /// Calling [disarm][Vm::disarm] must not be done until all referenced
    /// produced through these methods are no longer live.
    pub unsafe fn unsafe_into_ref<'out>(this: Self) -> &'out T {
        (*this.guards.get()).push(this.slot);
        let value = &*(this.value as *const _);
        mem::forget(this);
        value
    }
}

impl<T: ?Sized> ops::Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        self.access.release_shared();
    }
}

impl<T: ?Sized> fmt::Debug for Ref<'_, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.value, fmt)
    }
}

/// Guard for a value exclusively borrowed from a slot in the virtual machine.
///
/// These guards are necessary, since we need to guarantee certain forms of
/// access depending on what we do. Releasing the guard releases the access.
///
/// These also aid in function call integration, since they can be "arm" the
/// virtual machine to release shared guards through its unsafe functions.
///
/// See [disarm][Vm::disarm] for more information.
pub struct Mut<'a, T: ?Sized> {
    pub(super) value: &'a mut T,
    pub(super) access: &'a Access,
    pub(super) slot: Slot,
    pub(super) guards: &'a UnsafeCell<Vec<Slot>>,
}

impl<T: ?Sized> Mut<'_, T> {
    /// Convert into a reference with an unbounded lifetime.
    ///
    /// # Safety
    ///
    /// The returned reference must not outlive the VM that produced it.
    /// Calling [disarm][Vm::disarm] must not be done until all referenced
    /// produced through these methods are no longer live.
    pub unsafe fn unsafe_into_mut<'out>(this: Self) -> &'out mut T {
        (*this.guards.get()).push(this.slot);
        let value = &mut *(this.value as *mut _);
        mem::forget(this);
        value
    }
}

impl<T: ?Sized> ops::Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> ops::DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<T: ?Sized> Drop for Mut<'_, T> {
    fn drop(&mut self) {
        self.access.release_exclusive();
    }
}

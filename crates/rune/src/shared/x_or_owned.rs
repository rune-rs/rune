use std::ptr;

/// An unsafe container that can either unsafely hold a reference through an
/// unsafe API or an owned type.
#[repr(transparent)]
pub(crate) struct RefOrOwned<T> {
    kind: RefOrOwnedKind<T>,
}

enum RefOrOwnedKind<T> {
    Ptr(ptr::NonNull<T>),
    Owned(T),
}

impl<T> RefOrOwned<T> {
    /// Construct from an owned value.
    pub(crate) fn from_owned(value: T) -> Self {
        Self {
            kind: RefOrOwnedKind::Owned(value),
        }
    }

    /// Construct from a reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that this struct is not used after the data it's
    /// pointing to has been freed.
    pub(crate) unsafe fn from_ref(value: &T) -> Self {
        Self {
            kind: RefOrOwnedKind::Ptr(value.into()),
        }
    }

    /// Get as reference.
    pub(crate) fn as_ref(&self) -> &T {
        // SAFETY: This is a private enum and can only safely be constructed in
        // contexts where we tightly control the lifetime of Storage.
        match &self.kind {
            RefOrOwnedKind::Ptr(ptr) => unsafe { ptr.as_ref() },
            RefOrOwnedKind::Owned(value) => value,
        }
    }
}

/// An unsafe container that can either unsafely hold an exlusive reference
/// through an unsafe API or an owned type.
#[repr(transparent)]
pub(crate) struct MutOrOwned<T> {
    kind: MutOrOwnedKind<T>,
}

enum MutOrOwnedKind<T> {
    Ptr(ptr::NonNull<T>),
    Owned(T),
}

impl<T> MutOrOwned<T> {
    /// Construct from an owned value.
    pub(crate) fn from_owned(value: T) -> Self {
        Self {
            kind: MutOrOwnedKind::Owned(value),
        }
    }

    /// Construct from a mutable reference.
    ///
    /// # Safety
    ///
    /// Caller must ensure that this struct is not used after the data it's
    /// pointing to has been freed.
    pub(crate) unsafe fn from_mut(value: &mut T) -> Self {
        Self {
            kind: MutOrOwnedKind::Ptr(value.into()),
        }
    }

    /// Get as reference.
    pub(crate) fn as_ref(&self) -> &T {
        // SAFETY: This is a private enum and can only safely be constructed in
        // contexts where we tightly control the lifetime of Storage.
        match &self.kind {
            MutOrOwnedKind::Ptr(ptr) => unsafe { ptr.as_ref() },
            MutOrOwnedKind::Owned(value) => value,
        }
    }

    /// Get as mutable reference.
    pub(crate) fn as_mut(&mut self) -> &mut T {
        // SAFETY: This is a private enum and can only safely be constructed in
        // contexts where we tightly control the lifetime of Storage.
        match &mut self.kind {
            MutOrOwnedKind::Ptr(ptr) => unsafe { ptr.as_mut() },
            MutOrOwnedKind::Owned(value) => value,
        }
    }
}

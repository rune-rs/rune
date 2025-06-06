use core::alloc::{Layout, LayoutError};
use core::cell::Cell;
use core::fmt;
use core::mem::{align_of, needs_drop, replace, size_of, take};
use core::ptr::{self, addr_of, addr_of_mut, NonNull};

use crate::alloc;
use crate::alloc::alloc::{Allocator, Global};
use crate::alloc::fmt::TryWrite;
use crate::hash::Hash;
use crate::runtime::{
    Access, AccessError, BorrowMut, BorrowRef, Formatter, IntoOutput, ProtocolCaller, Rtti,
    RttiKind, RuntimeError, Snapshot, TypeInfo, Value, VmError,
};
use crate::sync::Arc;

#[derive(Debug)]
pub(crate) enum AnySequenceTakeError {
    Alloc(alloc::Error),
    Access(AccessError),
}

impl fmt::Display for AnySequenceTakeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnySequenceTakeError::Access(error) => error.fmt(f),
            AnySequenceTakeError::Alloc(error) => error.fmt(f),
        }
    }
}

impl core::error::Error for AnySequenceTakeError {}

impl From<AccessError> for AnySequenceTakeError {
    #[inline]
    fn from(error: AccessError) -> Self {
        Self::Access(error)
    }
}

impl From<alloc::Error> for AnySequenceTakeError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self::Alloc(error)
    }
}

/// A sequence of dynamic value defined at runtime.
///
/// This is an allocation-optimized container which allows an interior slice of
/// data `T` to be checked for access and `H` to be immutably accessed inside of
/// a single reference-counted container.
pub struct AnySequence<H, T> {
    shared: NonNull<AnySequenceData<H, T>>,
}

impl<H, T> AnySequence<H, T> {
    /// A dynamic value inside of the virtual machine.
    pub(crate) fn new(
        rtti: H,
        it: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> alloc::Result<Self> {
        let it = it.into_iter();
        let this = Self::alloc(rtti, it.len())?;

        // Fill out the newly allocated container.
        unsafe {
            let data = AnySequenceData::as_data_ptr(this.shared);

            for (i, value) in it.enumerate() {
                data.add(i).write(value);
            }
        }

        Ok(this)
    }

    /// A dynamic value inside of the virtual machine.
    fn alloc(rtti: H, len: usize) -> alloc::Result<Self> {
        let layout = AnySequenceData::<H, T>::layout(len)?;

        let shared = Global.allocate(layout)?.cast::<AnySequenceData<H, T>>();

        // SAFETY: We've allocated space for both the shared header and the
        // trailing data.
        unsafe {
            shared.write(AnySequenceData {
                rtti,
                count: Cell::new(1),
                access: Access::new(),
                len,
                data: [],
            });
        }

        Ok(Self { shared })
    }

    /// Test if the value is sharable.
    #[inline]
    pub(crate) fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.shared.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    #[inline]
    pub(crate) fn is_writable(&self) -> bool {
        unsafe { self.shared.as_ref().access.is_exclusive() }
    }

    /// Get access snapshot of shared value.
    #[inline]
    pub(crate) fn snapshot(&self) -> Snapshot {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { self.shared.as_ref().access.snapshot() }
    }

    /// Get the size of the dynamic collection of values.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { self.shared.as_ref().len }
    }

    /// Get runtime type information of the dynamic value.
    #[inline]
    pub(crate) fn rtti(&self) -> &H {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { &self.shared.as_ref().rtti }
    }

    /// Borrow the interior data array by reference.
    #[inline]
    pub(crate) fn borrow_ref(&self) -> Result<BorrowRef<'_, [T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = AnySequenceData::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowRef::new(data, guard.into_raw()))
        }
    }

    /// Borrow the interior data array by mutable reference.
    #[inline]
    pub(crate) fn borrow_mut(&self) -> Result<BorrowMut<'_, [T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = AnySequenceData::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowMut::new(data, guard.into_raw()))
        }
    }

    /// Take the interior value and drop it if necessary.
    #[inline]
    pub(crate) fn drop(self) -> Result<(), AccessError> {
        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            let len = self.shared.as_ref().len;
            AnySequenceData::drop_values(self.shared, len);
            Ok(())
        }
    }
}

impl<H, T> AnySequence<H, T>
where
    H: Clone,
{
    /// Take the interior value and return a handle to the taken value.
    pub(crate) fn take(self) -> Result<Self, AnySequenceTakeError> {
        // SAFETY: We are checking the interior value for access before taking
        // it.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            let len = self.shared.as_ref().len;
            let new = Self::alloc(self.rtti().clone(), len)?;
            let from = AnySequenceData::as_data_ptr(self.shared);
            let to = AnySequenceData::as_data_ptr(new.shared);
            to.copy_from_nonoverlapping(from, len);
            Ok(new)
        }
    }
}

impl<H, T> Drop for AnySequence<H, T> {
    fn drop(&mut self) {
        // Decrement a shared value.
        unsafe {
            AnySequenceData::dec(self.shared);
        }
    }
}

impl<H, T> Clone for AnySequence<H, T> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We know that the inner value is live in this instance.
        unsafe {
            AnySequenceData::inc(self.shared);
        }

        Self {
            shared: self.shared,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if ptr::eq(self.shared.as_ptr(), source.shared.as_ptr()) {
            return;
        }

        let old = replace(&mut self.shared, source.shared);

        // SAFETY: We know that the inner value is live in both instances.
        unsafe {
            AnySequenceData::dec(old);
            AnySequenceData::inc(self.shared);
        }
    }
}

#[repr(C)]
struct AnySequenceData<H, T> {
    /// Run time type information of the shared value.
    rtti: H,
    /// Reference count.
    count: Cell<usize>,
    /// Access flags.
    access: Access,
    /// The size of the dynamic value.
    len: usize,
    /// Start of data pointer. Only used for alignment.
    data: [T; 0],
}

impl<H, T> AnySequenceData<H, T> {
    #[inline]
    fn layout(len: usize) -> Result<Layout, LayoutError> {
        let array = Layout::array::<T>(len)?;
        Layout::from_size_align(
            size_of::<AnySequenceData<H, T>>() + array.size(),
            align_of::<AnySequenceData<H, T>>(),
        )
    }

    /// Get the rtti pointer in the shared container.
    #[inline]
    unsafe fn as_rtti_ptr(this: NonNull<Self>) -> NonNull<H> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).rtti))
    }

    /// Get the data pointer in the shared container.
    #[inline]
    unsafe fn as_data_ptr(this: NonNull<Self>) -> NonNull<T> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).data)).cast::<T>()
    }

    /// Increment the reference count of the inner value.
    #[inline]
    unsafe fn inc(this: NonNull<Self>) {
        let count_ref = &*addr_of!((*this.as_ptr()).count);
        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        if count == usize::MAX {
            crate::alloc::abort();
        }

        count_ref.set(count + 1);
    }

    /// Decrement the reference count in inner, and free the underlying data if
    /// it has reached zero.
    ///
    /// # Safety
    ///
    /// ProtocolCaller needs to ensure that `this` is a valid pointer.
    #[inline]
    unsafe fn dec(this: NonNull<Self>) {
        let count_ref = &*addr_of!((*this.as_ptr()).count);
        let access = &*addr_of!((*this.as_ptr()).access);
        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        let count = count - 1;
        count_ref.set(count);

        if count != 0 {
            return;
        }

        let len = (*this.as_ptr()).len;

        let Ok(layout) = Self::layout(len) else {
            unreachable!();
        };

        if !access.is_taken() {
            Self::drop_values(this, len);
        }

        if needs_drop::<H>() {
            Self::as_rtti_ptr(this).drop_in_place();
        }

        Global.deallocate(this.cast(), layout);
    }

    #[inline]
    unsafe fn drop_values(this: NonNull<Self>, len: usize) {
        if needs_drop::<T>() {
            let data = Self::as_data_ptr(this);
            NonNull::slice_from_raw_parts(data, len).drop_in_place();
        }
    }
}

impl<T> AnySequence<Arc<Rtti>, T> {
    /// Access type hash on the dynamic value.
    #[inline]
    pub(crate) fn type_hash(&self) -> Hash {
        self.rtti().hash
    }

    /// Access type information on the dynamic value.
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        Rtti::type_info(self.rtti().clone())
    }

    /// Access a field by name.
    #[inline]
    pub(crate) fn get_field_ref(&self, key: &str) -> Result<Option<BorrowRef<'_, T>>, AccessError> {
        let Some(index) = self.rtti().fields.get(key) else {
            return Ok(None);
        };

        self.get_ref(*index)
    }

    /// Access a field mutably by name.
    #[inline]
    pub(crate) fn get_field_mut(&self, key: &str) -> Result<Option<BorrowMut<'_, T>>, AccessError> {
        let Some(index) = self.rtti().fields.get(key) else {
            return Ok(None);
        };

        self.get_mut(*index)
    }

    /// Access a field by index.
    #[inline]
    pub(crate) fn get_ref(&self, index: usize) -> Result<Option<BorrowRef<'_, T>>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let shared = self.shared.as_ref();

            if index >= shared.len {
                return Ok(None);
            }

            let guard = shared.access.shared()?;
            let data = AnySequenceData::as_data_ptr(self.shared).add(index);
            Ok(Some(BorrowRef::new(data, guard.into_raw())))
        }
    }

    /// Access a field mutably by index.
    #[inline]
    pub(crate) fn get_mut(&self, index: usize) -> Result<Option<BorrowMut<'_, T>>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let shared = self.shared.as_ref();

            if index >= shared.len {
                return Ok(None);
            }

            let guard = shared.access.exclusive()?;
            let data = AnySequenceData::as_data_ptr(self.shared).add(index);
            Ok(Some(BorrowMut::new(data, guard.into_raw())))
        }
    }
}

impl AnySequence<Arc<Rtti>, Value> {
    /// Debug print the dynamic value.
    pub(crate) fn debug_fmt_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> Result<(), VmError> {
        let rtti = self.rtti();
        let values = self.borrow_ref()?;

        match rtti.kind {
            RttiKind::Empty => debug_empty(rtti, f),
            RttiKind::Tuple => debug_tuple(rtti, &values, f, caller),
            RttiKind::Struct => debug_struct(rtti, &values, f, caller),
        }
    }
}

fn debug_empty(rtti: &Rtti, f: &mut Formatter) -> Result<(), VmError> {
    write!(f, "{}", rtti.item)?;
    Ok(())
}

fn debug_tuple(
    rtti: &Rtti,
    values: &[Value],
    f: &mut Formatter,
    caller: &mut dyn ProtocolCaller,
) -> Result<(), VmError> {
    write!(f, "{} (", rtti.item)?;

    let mut first = true;

    for value in values.iter() {
        if !take(&mut first) {
            write!(f, ", ")?;
        }

        value.debug_fmt_with(f, caller)?;
    }

    write!(f, ")")?;
    Ok(())
}

fn debug_struct(
    rtti: &Rtti,
    values: &[Value],
    f: &mut Formatter,
    caller: &mut dyn ProtocolCaller,
) -> Result<(), VmError> {
    write!(f, "{} {{", rtti.item)?;

    let mut first = true;

    for (index, field) in values.iter().enumerate() {
        let Some((name, _)) = rtti.fields.iter().find(|t| *t.1 == index) else {
            continue;
        };

        if !take(&mut first) {
            write!(f, ", ")?;
        }

        write!(f, "{name}: ")?;
        field.debug_fmt_with(f, caller)?;
    }

    write!(f, "}}")?;
    Ok(())
}

impl IntoOutput for AnySequence<Arc<Rtti>, Value> {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(Value::from(self))
    }
}

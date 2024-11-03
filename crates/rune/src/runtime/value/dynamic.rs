use core::alloc::{Layout, LayoutError};
use core::cell::Cell;
use core::fmt;
use core::mem::{align_of, needs_drop, replace, size_of, take};
use core::ptr::{self, addr_of, addr_of_mut, NonNull};

use rust_alloc::sync::Arc;

use crate::alloc;
use crate::alloc::alloc::{Allocator, Global};
use crate::alloc::fmt::TryWrite;
use crate::hash::Hash;
use crate::runtime::{
    Access, AccessError, BorrowMut, BorrowRef, Formatter, IntoOutput, ProtocolCaller, Rtti,
    RttiKind, Snapshot, TypeInfo, Value, VmResult,
};

#[derive(Debug)]
pub(crate) enum DynamicTakeError {
    Access(AccessError),
    Alloc(alloc::Error),
}

impl fmt::Display for DynamicTakeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynamicTakeError::Access(error) => error.fmt(f),
            DynamicTakeError::Alloc(error) => error.fmt(f),
        }
    }
}

impl core::error::Error for DynamicTakeError {}

impl From<AccessError> for DynamicTakeError {
    fn from(error: AccessError) -> Self {
        Self::Access(error)
    }
}

impl From<alloc::Error> for DynamicTakeError {
    fn from(error: alloc::Error) -> Self {
        Self::Alloc(error)
    }
}

/// A dynamic value defined at runtime.
///
/// This is an allocation-optimized container which allows an interior slice of
/// data `T` to be checked for access and `H` to be immutably accessed inside of
/// a single reference-counted container.
pub struct Dynamic<H, T> {
    shared: NonNull<Shared<H, T>>,
}

impl<H, T> Dynamic<H, T> {
    /// A dynamic value inside of the virtual machine.
    pub(crate) fn new(
        rtti: H,
        it: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> alloc::Result<Self> {
        let it = it.into_iter();
        let this = Self::alloc(rtti, it.len())?;

        // Fill out the newly allocated container.
        unsafe {
            let data = Shared::as_data_ptr(this.shared);

            for (i, value) in it.enumerate() {
                data.add(i).write(value);
            }
        }

        Ok(this)
    }

    /// A dynamic value inside of the virtual machine.
    fn alloc(rtti: H, len: usize) -> alloc::Result<Self> {
        let layout = Shared::<H, T>::layout(len)?;

        let shared = Global.allocate(layout)?.cast::<Shared<H, T>>();

        // SAFETY: We've allocated space for both the shared header and the
        // trailing data.
        unsafe {
            shared.write(Shared {
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
    pub(crate) fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.shared.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    pub(crate) fn is_writable(&self) -> bool {
        unsafe { self.shared.as_ref().access.is_exclusive() }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { self.shared.as_ref().access.snapshot() }
    }

    /// Get the size of the dynamic collection of values.
    pub(crate) fn len(&self) -> usize {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { self.shared.as_ref().len }
    }

    /// Get runtime type information of the dynamic value.
    pub(crate) fn rtti(&self) -> &H {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { &self.shared.as_ref().rtti }
    }

    /// Borrow the interior data array by reference.
    pub(crate) fn borrow_ref(&self) -> Result<BorrowRef<[T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = Shared::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowRef::new(data, guard.into_raw()))
        }
    }

    /// Borrow the interior data array by mutable reference.
    pub(crate) fn borrow_mut(&self) -> Result<BorrowMut<[T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = Shared::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowMut::new(data, guard.into_raw()))
        }
    }

    /// Take the interior value and drop it if necessary.
    pub(crate) fn drop(self) -> Result<(), AccessError> {
        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            let len = self.shared.as_ref().len;
            Shared::drop_values(self.shared, len);
            Ok(())
        }
    }
}

impl<H, T> Dynamic<H, T>
where
    H: Clone,
{
    /// Take the interior value and return a handle to the taken value.
    pub(crate) fn take(self) -> Result<Self, DynamicTakeError> {
        // SAFETY: We are checking the interior value for access before taking
        // it.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            let len = self.shared.as_ref().len;
            let new = Self::alloc(self.rtti().clone(), len)?;
            let from = Shared::as_data_ptr(self.shared);
            let to = Shared::as_data_ptr(new.shared);
            to.copy_from_nonoverlapping(from, len);
            Ok(new)
        }
    }
}

impl<H, T> Drop for Dynamic<H, T> {
    fn drop(&mut self) {
        // Decrement a shared value.
        unsafe {
            Shared::dec(self.shared);
        }
    }
}

impl<H, T> Clone for Dynamic<H, T> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We know that the inner value is live in this instance.
        unsafe {
            Shared::inc(self.shared);
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
            Shared::dec(old);
            Shared::inc(self.shared);
        }
    }
}

struct Shared<H, T> {
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

impl<H, T> Shared<H, T> {
    fn layout(len: usize) -> Result<Layout, LayoutError> {
        let array = Layout::array::<T>(len)?;
        Layout::from_size_align(
            size_of::<Shared<H, T>>() + array.size(),
            align_of::<Shared<H, T>>(),
        )
    }

    /// Get the rtti pointer in the shared container.
    unsafe fn as_rtti_ptr(this: NonNull<Self>) -> NonNull<H> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).rtti))
    }

    /// Get the data pointer in the shared container.
    unsafe fn as_data_ptr(this: NonNull<Self>) -> NonNull<T> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).data)).cast::<T>()
    }

    /// Increment the reference count of the inner value.
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

    unsafe fn drop_values(this: NonNull<Self>, len: usize) {
        if needs_drop::<T>() {
            let data = Self::as_data_ptr(this);
            NonNull::slice_from_raw_parts(data, len).drop_in_place();
        }
    }
}

impl<T> Dynamic<Arc<Rtti>, T> {
    /// Access type hash on the dynamic value.
    #[inline]
    pub(crate) fn type_hash(&self) -> Hash {
        self.rtti().hash
    }

    /// Access type information on the dynamic value.
    #[inline]
    pub(crate) fn type_info(&self) -> TypeInfo {
        self.rtti().clone().type_info()
    }

    /// Access a field by name.
    #[inline]
    pub(crate) fn get_field_ref(&self, key: &str) -> Result<Option<BorrowRef<'_, T>>, AccessError> {
        let Some(index) = self.rtti().fields.get(key) else {
            return Ok(None);
        };

        let values = self.borrow_ref()?;

        let index = *index;
        let value = BorrowRef::try_map(values, |value| value.get(index));
        Ok(value.ok())
    }

    /// Access a field mutably by name.
    #[inline]
    pub(crate) fn get_field_mut(&self, key: &str) -> Result<Option<BorrowMut<'_, T>>, AccessError> {
        let Some(index) = self.rtti().fields.get(key) else {
            return Ok(None);
        };

        let values = self.borrow_mut()?;

        let index = *index;
        let value = BorrowMut::try_map(values, |value| value.get_mut(index));
        Ok(value.ok())
    }

    /// Access a field by index.
    #[inline]
    pub(crate) fn get_ref(&self, index: usize) -> Result<Option<BorrowRef<'_, T>>, AccessError> {
        let values = self.borrow_ref()?;
        let value = BorrowRef::try_map(values, |value| value.get(index));
        Ok(value.ok())
    }

    /// Access a field mutably by index.
    #[inline]
    pub(crate) fn get_mut(&self, index: usize) -> Result<Option<BorrowMut<'_, T>>, AccessError> {
        let values = self.borrow_mut()?;
        let value = BorrowMut::try_map(values, |value| value.get_mut(index));
        Ok(value.ok())
    }
}

impl Dynamic<Arc<Rtti>, Value> {
    /// Debug print the dynamic value.
    pub(crate) fn debug_fmt_with(
        &self,
        f: &mut Formatter,
        caller: &mut dyn ProtocolCaller,
    ) -> VmResult<()> {
        let rtti = self.rtti();
        let values = vm_try!(self.borrow_ref());

        match rtti.kind {
            RttiKind::Empty => debug_empty(rtti, f),
            RttiKind::Tuple => debug_tuple(rtti, &values, f, caller),
            RttiKind::Struct => debug_struct(rtti, &values, f, caller),
        }
    }
}

fn debug_empty(rtti: &Rtti, f: &mut Formatter) -> VmResult<()> {
    vm_try!(write!(f, "{}", rtti.item));
    VmResult::Ok(())
}

fn debug_tuple(
    rtti: &Rtti,
    values: &[Value],
    f: &mut Formatter,
    caller: &mut dyn ProtocolCaller,
) -> VmResult<()> {
    vm_try!(write!(f, "{} (", rtti.item));

    let mut first = true;

    for value in values.iter() {
        if !take(&mut first) {
            vm_try!(write!(f, ", "));
        }

        vm_try!(value.debug_fmt_with(f, caller));
    }

    vm_try!(write!(f, ")"));
    VmResult::Ok(())
}

fn debug_struct(
    rtti: &Rtti,
    values: &[Value],
    f: &mut Formatter,
    caller: &mut dyn ProtocolCaller,
) -> VmResult<()> {
    vm_try!(write!(f, "{} {{", rtti.item));

    let mut first = true;

    for (index, field) in values.iter().enumerate() {
        let Some((name, _)) = rtti.fields.iter().find(|t| *t.1 == index) else {
            continue;
        };

        if !take(&mut first) {
            vm_try!(write!(f, ", "));
        }

        vm_try!(write!(f, "{name}: "));
        vm_try!(field.debug_fmt_with(f, caller));
    }

    vm_try!(write!(f, "}}"));
    VmResult::Ok(())
}

impl IntoOutput for Dynamic<Arc<Rtti>, Value> {
    type Output = Dynamic<Arc<Rtti>, Value>;

    #[inline]
    fn into_output(self) -> VmResult<Self::Output> {
        VmResult::Ok(self)
    }
}

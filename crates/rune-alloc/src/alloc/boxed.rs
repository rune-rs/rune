use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::ops::{Deref, DerefMut};

use crate::alloc::raw_vec::RawVec;
use crate::alloc::{AllocError, Allocator, Error, Global, TryClone, Vec};
use crate::ptr;
use crate::ptr::Unique;

#[test]
fn ensure_niche_size() {
    assert_eq!(
        ::core::mem::size_of::<Option<Box<u32>>>(),
        ::core::mem::size_of::<Box<u32>>()
    );
}

/// A pointer type that uniquely owns a heap allocation of type `T`.
pub struct Box<T: ?Sized, A: Allocator = Global> {
    ptr: Unique<T>,
    alloc: A,
}

impl<T> Box<T, Global> {
    /// Allocates memory on the heap and then places `x` into it.
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::Box;
    ///
    /// let five = Box::new(5)?;
    /// # Ok::<_, rune_alloc::AllocError>(())
    /// ```
    pub fn new(value: T) -> Result<Self, AllocError> {
        Self::try_new_in(value, Global)
    }
}

impl<T, A: Allocator> Box<T, A> {
    /// Allocates memory in the given allocator then places `x` into it,
    /// returning an error if the allocation fails
    ///
    /// This doesn't actually allocate if `T` is zero-sized.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let five = Box::try_new_in(5, Global)?;
    /// # Ok::<(), rune_alloc::AllocError>(())
    /// ```
    #[inline]
    pub fn try_new_in(x: T, alloc: A) -> Result<Self, AllocError> {
        let mut boxed = Self::try_new_uninit_in(alloc)?;

        unsafe {
            boxed.as_mut_ptr().write(x);
            Ok(boxed.assume_init())
        }
    }

    /// Constructs a new box with uninitialized contents in the provided
    /// allocator, returning an error if the allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let mut five = Box::<u32>::try_new_uninit_in(Global)?;
    ///
    /// let five: Box<u32> = unsafe {
    ///     // Deferred initialization:
    ///     five.as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<_, rune_alloc::AllocError>(())
    /// ```
    pub fn try_new_uninit_in(alloc: A) -> Result<Box<mem::MaybeUninit<T>, A>, AllocError>
    where
        A: Allocator,
    {
        let layout = Layout::new::<mem::MaybeUninit<T>>();
        let ptr = alloc.allocate(layout)?.cast();
        unsafe { Ok(Box::from_raw_in(ptr.as_ptr(), alloc)) }
    }

    /// Converts a `Box<T>` into a `Box<[T]>`
    ///
    /// This conversion does not allocate on the heap and happens in place.
    pub(crate) fn into_boxed_slice(boxed: Self) -> Box<[T], A> {
        let (raw, alloc) = Box::into_raw_with_allocator(boxed);
        unsafe { Box::from_raw_in(raw as *mut [T; 1], alloc) }
    }

    /// Consumes the `Box`, returning the wrapped value.
    #[inline]
    pub fn into_inner(boxed: Self) -> T {
        let this = mem::ManuallyDrop::new(boxed);
        let value = unsafe { ptr::read(this.ptr.as_ptr()) };

        // Free memory associated with the box.
        //
        // SAFETY: We own the box, so we know we can safely deallocate it.
        unsafe {
            let layout = for_value_raw(this.ptr.as_ptr());

            if layout.size() != 0 {
                this.alloc.deallocate(From::from(this.ptr.cast()), layout);
            }
        }

        value
    }
}

impl<T: ?Sized, A: Allocator> Box<T, A> {
    /// Consumes and leaks the `Box`, returning a mutable reference, `&'a mut
    /// T`. Note that the type `T` must outlive the chosen lifetime `'a`. If the
    /// type has only static references, or none at all, then this may be chosen
    /// to be `'static`.
    ///
    /// This function is mainly useful for data that lives for the remainder of
    /// the program's life. Dropping the returned reference will cause a memory
    /// leak. If this is not acceptable, the reference should first be wrapped
    /// with the [`Box::from_raw_in`] function producing a `Box`. This `Box` can
    /// then be dropped which will properly destroy `T` and release the
    /// allocated memory.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `Box::leak(b)` instead of `b.leak()`. This is so that there is no
    /// conflict with a method on the inner type.
    ///
    /// # Examples
    ///
    /// Simple usage:
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune_alloc::Error> {
    /// use rune_alloc::Box;
    ///
    /// let x = Box::new(41)?;
    /// let static_ref: &'static mut usize = Box::leak(x);
    /// *static_ref += 1;
    /// assert_eq!(*static_ref, 42);
    /// # Ok::<_, rune_alloc::Error>(()) }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    ///
    /// Unsized data:
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune_alloc::Error> {
    /// use rune_alloc::Box;
    ///
    /// let x = rune_alloc::try_vec![1, 2, 3].try_into_boxed_slice()?;
    /// let static_ref = Box::leak(x);
    /// static_ref[0] = 4;
    /// assert_eq!(*static_ref, [4, 2, 3]);
    /// # Ok::<_, rune_alloc::Error>(()) }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    #[inline]
    pub fn leak<'a>(b: Self) -> &'a mut T
    where
        A: 'a,
    {
        unsafe { &mut *mem::ManuallyDrop::new(b).ptr.as_ptr() }
    }

    /// Constructs a box from a raw pointer in the given allocator.
    ///
    /// After calling this function, the raw pointer is owned by the resulting
    /// `Box`. Specifically, the `Box` destructor will call the destructor of
    /// `T` and free the allocated memory. For this to be safe, the memory must
    /// have been allocated in accordance with the [memory layout] used by `Box`
    /// .
    ///
    /// # Safety
    ///
    /// This function is unsafe because improper use may lead to memory
    /// problems. For example, a double-free may occur if the function is called
    /// twice on the same raw pointer.
    ///
    /// # Examples
    ///
    /// Recreate a `Box` which was previously converted to a raw pointer using
    /// [`Box::into_raw_with_allocator`]:
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let x = Box::try_new_in(5, Global)?;
    /// let (ptr, alloc) = Box::into_raw_with_allocator(x);
    /// let x = unsafe { Box::from_raw_in(ptr, alloc) };
    /// # Ok::<(), rune_alloc::AllocError>(())
    /// ```
    ///
    /// Manually create a `Box` from scratch by using the system allocator:
    ///
    /// ```
    /// use core::alloc::Layout;
    /// use rune_alloc::{Box, Allocator, Global};
    ///
    /// unsafe {
    ///     let ptr = Global.allocate(Layout::new::<i32>())?.as_ptr() as *mut i32;
    ///     // In general .write is required to avoid attempting to destruct
    ///     // the (uninitialized) previous contents of `ptr`, though for this
    ///     // simple example `*ptr = 5` would have worked as well.
    ///     ptr.write(5);
    ///     let x = Box::from_raw_in(ptr, Global);
    /// }
    /// # Ok::<(), rune_alloc::AllocError>(())
    /// ```
    ///
    /// [memory layout]: self#memory-layout
    /// [`Layout`]: crate::Layout
    #[inline]
    pub unsafe fn from_raw_in(raw: *mut T, alloc: A) -> Self {
        Self {
            ptr: unsafe { Unique::new_unchecked(raw) },
            alloc,
        }
    }

    /// Consumes the `Box`, returning a wrapped raw pointer and the allocator.
    ///
    /// The pointer will be properly aligned and non-null.
    ///
    /// After calling this function, the caller is responsible for the
    /// memory previously managed by the `Box`. In particular, the
    /// caller should properly destroy `T` and release the memory, taking
    /// into account the [memory layout] used by `Box`. The easiest way to
    /// do this is to convert the raw pointer back into a `Box` with the
    /// [`Box::from_raw_in`] function, allowing the `Box` destructor to perform
    /// the cleanup.
    ///
    /// Note: this is an associated function, which means that you have
    /// to call it as `Box::into_raw_with_allocator(b)` instead of `b.into_raw_with_allocator()`. This
    /// is so that there is no conflict with a method on the inner type.
    ///
    /// # Examples
    ///
    /// Converting the raw pointer back into a `Box` with [`Box::from_raw_in`]
    /// for automatic cleanup:
    ///
    /// ```
    /// use rune_alloc::{Box, String, Global};
    ///
    /// let x = Box::try_new_in(String::try_from("Hello")?, Global)?;
    /// let (ptr, alloc) = Box::into_raw_with_allocator(x);
    /// let x = unsafe { Box::from_raw_in(ptr, alloc) };
    /// # Ok::<_, rune_alloc::Error>(())
    /// ```
    ///
    /// Manual cleanup by explicitly running the destructor and deallocating the
    /// memory:
    ///
    /// ```
    /// use core::alloc::Layout;
    /// use core::ptr::{self, NonNull};
    /// use rune_alloc::{Allocator, Box, String, Global};
    ///
    /// let x = Box::try_new_in(String::try_from("Hello")?, Global)?;
    ///
    /// let (ptr, alloc) = Box::into_raw_with_allocator(x);
    ///
    /// unsafe {
    ///     ptr::drop_in_place(ptr);
    ///     let non_null = NonNull::new_unchecked(ptr);
    ///     alloc.deallocate(non_null.cast(), Layout::new::<String>());
    /// }
    /// # Ok::<_, rune_alloc::Error>(())
    /// ```
    ///
    /// [memory layout]: self#memory-layout
    #[inline]
    pub fn into_raw_with_allocator(b: Self) -> (*mut T, A) {
        let leaked = mem::ManuallyDrop::new(b);
        // SAFETY: We prevent the alloc field from being dropped, so we can
        // safely smuggle it out.
        let alloc = unsafe { ptr::read(&leaked.alloc) };
        (leaked.ptr.as_ptr(), alloc)
    }
}

impl<T, A: Allocator> Box<mem::MaybeUninit<T>, A> {
    /// Converts to `Box<T, A>`.
    ///
    /// # Safety
    ///
    /// As with [`MaybeUninit::assume_init`],
    /// it is up to the caller to guarantee that the value
    /// really is in an initialized state.
    /// Calling this when the content is not yet fully initialized
    /// causes immediate undefined behavior.
    ///
    /// [`MaybeUninit::assume_init`]: mem::MaybeUninit::assume_init
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let mut five = Box::<u32>::try_new_uninit_in(Global)?;
    ///
    /// let five: Box<u32> = unsafe {
    ///     // Deferred initialization:
    ///     five.as_mut_ptr().write(5);
    ///
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<_, rune_alloc::AllocError>(())
    /// ```
    #[inline]
    pub unsafe fn assume_init(self) -> Box<T, A> {
        let (raw, alloc) = Box::into_raw_with_allocator(self);
        unsafe { Box::from_raw_in(raw as *mut T, alloc) }
    }
}

impl<T, A: Allocator> Box<[T], A> {
    /// Constructs a new boxed slice with uninitialized contents. Returns an error if
    /// the allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let mut values = Box::<[u32]>::try_new_uninit_slice_in(3, Global)?;
    ///
    /// let values = unsafe {
    ///     // Deferred initialization:
    ///     values[0].as_mut_ptr().write(1);
    ///     values[1].as_mut_ptr().write(2);
    ///     values[2].as_mut_ptr().write(3);
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # Ok::<_, rune_alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_new_uninit_slice_in(
        len: usize,
        alloc: A,
    ) -> Result<Box<[mem::MaybeUninit<T>], A>, Error> {
        unsafe {
            let layout = match Layout::array::<mem::MaybeUninit<T>>(len) {
                Ok(l) => l,
                Err(_) => return Err(Error::LayoutError),
            };
            let ptr = alloc.allocate(layout)?;
            Ok(RawVec::from_raw_parts_in(ptr.as_ptr() as *mut _, len, alloc).into_box(len))
        }
    }
}

impl<T, A: Allocator> Box<[mem::MaybeUninit<T>], A> {
    /// Converts to `Box<[T], A>`.
    ///
    /// # Safety
    ///
    /// As with [`MaybeUninit::assume_init`],
    /// it is up to the caller to guarantee that the values
    /// really are in an initialized state.
    /// Calling this when the content is not yet fully initialized
    /// causes immediate undefined behavior.
    ///
    /// [`MaybeUninit::assume_init`]: mem::MaybeUninit::assume_init
    ///
    /// # Examples
    ///
    /// ```
    /// use rune_alloc::{Box, Global};
    ///
    /// let mut values = Box::<[u32]>::try_new_uninit_slice_in(3, Global)?;
    ///
    /// let values = unsafe {
    ///     // Deferred initialization:
    ///     values[0].as_mut_ptr().write(1);
    ///     values[1].as_mut_ptr().write(2);
    ///     values[2].as_mut_ptr().write(3);
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # Ok::<_, rune_alloc::Error>(())
    /// ```
    #[inline]
    pub unsafe fn assume_init(self) -> Box<[T], A> {
        let (raw, alloc) = Box::into_raw_with_allocator(self);
        unsafe { Box::from_raw_in(raw as *mut [T], alloc) }
    }
}

impl<T, A: Allocator + Clone> TryClone for Box<T, A>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        let value = (**self).try_clone()?;
        let alloc = self.alloc.clone();
        Ok(Box::try_new_in(value, alloc)?)
    }
}

impl<T, A: Allocator + Clone> TryClone for Box<[T], A>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        let alloc = self.alloc.clone();
        let vec = crate::alloc::slice::to_vec(self, alloc)?;
        vec.try_into_boxed_slice()
    }
}

impl<T: ?Sized, A: Allocator> Deref for Box<T, A> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized, A: Allocator> DerefMut for Box<T, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized, A: Allocator> Drop for Box<T, A> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let ptr = self.ptr;

            if mem::needs_drop::<T>() {
                ptr::drop_in_place(ptr.as_ptr());
            }

            let layout = for_value_raw(ptr.as_ptr());

            if layout.size() != 0 {
                self.alloc.deallocate(From::from(ptr.cast()), layout);
            }
        }
    }
}

impl<T> Default for Box<[T], Global> {
    fn default() -> Self {
        Box {
            ptr: Unique::dangling_empty_slice(),
            alloc: Global,
        }
    }
}

impl<T: ?Sized, A: Allocator> fmt::Display for Box<T, A>
where
    T: fmt::Display,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized, A: Allocator> fmt::Debug for Box<T, A>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<A: Allocator> From<Box<str, A>> for Box<[u8], A> {
    fn from(value: Box<str, A>) -> Self {
        // SAFETY: `[u8]` is layout compatible with `str` and there are no
        // checks needed.
        unsafe {
            let (ptr, alloc) = Box::into_raw_with_allocator(value);
            Box::from_raw_in(ptr as *mut [u8], alloc)
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> TryFrom<::rust_alloc::boxed::Box<[T]>> for Box<[T]> {
    type Error = Error;

    #[inline]
    fn try_from(values: ::rust_alloc::boxed::Box<[T]>) -> Result<Self, Error> {
        let mut vec = Vec::try_with_capacity(values.len())?;

        for value in ::rust_alloc::vec::Vec::from(values) {
            vec.try_push(value)?;
        }

        vec.try_into_boxed_slice()
    }
}

impl<T, const N: usize> TryFrom<[T; N]> for Box<[T]> {
    type Error = Error;

    #[inline]
    fn try_from(values: [T; N]) -> Result<Self, Error> {
        let mut vec = Vec::try_with_capacity(values.len())?;

        for value in values {
            vec.try_push(value)?;
        }

        vec.try_into_boxed_slice()
    }
}

unsafe fn for_value_raw<T: ?Sized>(t: *const T) -> Layout {
    // SAFETY: we pass along the prerequisites of these functions to the caller
    // TODO: Use mem::{size_of_val_raw, align_of_val_raw} when they become
    // stable, for now we privately know that this can safely be turned into a
    // reference since it's only used while dropping an owned value of type `T`.
    let (size, align) = (mem::size_of_val(&*t), mem::align_of_val(&*t));
    // SAFETY: see rationale in `new` for why this is using the unsafe variant
    Layout::from_size_align_unchecked(size, align)
}

//! The `Box<T>` type for heap allocation.
//!
//! [`Box<T>`], casually referred to as a 'box', provides the simplest form of
//! heap allocation in Rust. Boxes provide ownership for this allocation, and
//! drop their contents when they go out of scope. Boxes also ensure that they
//! never allocate more than `isize::MAX` bytes.
//!
//! # Examples
//!
//! Move a value from the stack to the heap by creating a [`Box`]:
//!
//! ```
//! use rune::alloc::Box;
//!
//! let val: u8 = 5;
//! let boxed: Box<u8> = Box::try_new(val)?;
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! Move a value from a [`Box`] back to the stack using [Box::into_inner]:
//!
//! ```
//! use rune::alloc::Box;
//!
//! let boxed: Box<u8> = Box::try_new(5)?;
//! let val: u8 = Box::into_inner(boxed);
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! Creating a recursive data structure:
//!
//! ```
//! use rune::alloc::Box;
//!
//! #[derive(Debug)]
//! enum List<T> {
//!     Cons(T, Box<List<T>>),
//!     Nil,
//! }
//!
//! let list: List<i32> = List::Cons(1, Box::try_new(List::Cons(2, Box::try_new(List::Nil)?))?);
//! println!("{list:?}");
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! This will print `Cons(1, Cons(2, Nil))`.
//!
//! Recursive structures must be boxed, because if the definition of `Cons`
//! looked like this:
//!
//! ```compile_fail,E0072
//! # enum List<T> {
//! Cons(T, List<T>),
//! # }
//! ```
//!
//! It wouldn't work. This is because the size of a `List` depends on how many
//! elements are in the list, and so we don't know how much memory to allocate
//! for a `Cons`. By introducing a [`Box<T>`], which has a defined size, we know
//! how big `Cons` needs to be.
//!
//! # Memory layout
//!
//! For non-zero-sized values, a [`Box`] will use the [`Global`] allocator for
//! its allocation. It is valid to convert both ways between a [`Box`] and a raw
//! pointer allocated with the [`Global`] allocator, given that the [`Layout`]
//! used with the allocator is correct for the type. More precisely, a `value:
//! *mut T` that has been allocated with the [`Global`] allocator with
//! `Layout::for_value(&*value)` may be converted into a box using
//! [`Box::<T>::from_raw_in(value)`]. Conversely, the memory backing a `value:
//! *mut T` obtained from [`Box::<T>::into_raw_with_allocator`] may be
//! deallocated using the [`Global`] allocator with
//! [`Layout::for_value(&*value)`].
//!
//! For zero-sized values, the `Box` pointer still has to be [valid] for reads
//! and writes and sufficiently aligned. In particular, casting any aligned
//! non-zero integer literal to a raw pointer produces a valid pointer, but a
//! pointer pointing into previously allocated memory that since got freed is
//! not valid. The recommended way to build a Box to a ZST if `Box::new` cannot
//! be used is to use [`ptr::NonNull::dangling`].
//!
//! So long as `T: Sized`, a `Box<T>` is guaranteed to be represented as a
//! single pointer and is also ABI-compatible with C pointers (i.e. the C type
//! `T*`). This means that if you have extern "C" Rust functions that will be
//! called from C, you can define those Rust functions using `Box<T>` types, and
//! use `T*` as corresponding type on the C side. As an example, consider this C
//! header which declares functions that create and destroy some kind of `Foo`
//! value:
//!
//! ```c
//! /* C header */
//!
//! /* Returns ownership to the caller */
//! struct Foo* foo_new(void);
//!
//! /* Takes ownership from the caller; no-op when invoked with null */
//! void foo_delete(struct Foo*);
//! ```
//!
//! These two functions might be implemented in Rust as follows. Here, the
//! `struct Foo*` type from C is translated to `Box<Foo>`, which captures the
//! ownership constraints. Note also that the nullable argument to `foo_delete`
//! is represented in Rust as `Option<Box<Foo>>`, since `Box<Foo>` cannot be
//! null.
//!
//! ```
//! use rune::alloc::Box;
//! use rune::alloc::alloc::AllocError;
//!
//! #[repr(C)]
//! pub struct Foo;
//!
//! #[no_mangle]
//! pub extern "C" fn foo_new() -> Result<Box<Foo>, AllocError> {
//!     Box::try_new(Foo)
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn foo_delete(_: Option<Box<Foo>>) {}
//! ```
//!
//! Even though `Box<T>` has the same representation and C ABI as a C pointer,
//! this does not mean that you can convert an arbitrary `T*` into a `Box<T>`
//! and expect things to work. `Box<T>` values will always be fully aligned,
//! non-null pointers. Moreover, the destructor for `Box<T>` will attempt to
//! free the value with the global allocator. In general, the best practice is
//! to only use `Box<T>` for pointers that originated from the global allocator.
//!
//! **Important.** At least at present, you should avoid using `Box<T>` types
//! for functions that are defined in C but invoked from Rust. In those cases,
//! you should directly mirror the C types as closely as possible. Using types
//! like `Box<T>` where the C definition is just using `T*` can lead to
//! undefined behavior, as described in
//! [rust-lang/unsafe-code-guidelines#198][ucg#198].
//!
//! # Considerations for unsafe code
//!
//! **Warning: This section is not normative and is subject to change, possibly
//! being relaxed in the future! It is a simplified summary of the rules
//! currently implemented in the compiler.**
//!
//! The aliasing rules for `Box<T>` are the same as for `&mut T`. `Box<T>`
//! asserts uniqueness over its content. Using raw pointers derived from a box
//! after that box has been mutated through, moved or borrowed as `&mut T` is
//! not allowed. For more guidance on working with box from unsafe code, see
//! [rust-lang/unsafe-code-guidelines#326][ucg#326].
//!
//!
//! [ucg#198]: https://github.com/rust-lang/unsafe-code-guidelines/issues/198
//! [ucg#326]: https://github.com/rust-lang/unsafe-code-guidelines/issues/326
//! [dereferencing]: core::ops::Deref
//! [`Box::<T>::from_raw_in(value)`]: Box::from_raw_in
//! [`Global`]: crate::alloc::Global
//! [`Layout`]: core::alloc::Layout
//! [`Layout::for_value(&*value)`]: core::alloc::Layout::for_value
//! [valid]: core::ptr#safety

use core::alloc::Layout;
use core::borrow::{Borrow, BorrowMut};
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::mem;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;

use crate::alloc::{AllocError, Allocator, Global};
use crate::clone::TryClone;
use crate::error::Error;
use crate::iter::TryFromIteratorIn;
use crate::path::Path;
use crate::ptr::{self, Unique};
use crate::raw_vec::RawVec;
use crate::vec::Vec;

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
    /// use rune::alloc::Box;
    ///
    /// let five = Box::try_new(5)?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_new(value: T) -> Result<Self, AllocError> {
        Self::try_new_in(value, Global)
    }

    /// Constructs a new `Pin<Box<T>>`. If `T` does not implement [`Unpin`],
    /// then `x` will be pinned in memory and unable to be moved.
    ///
    /// Constructing and pinning of the `Box` can also be done in two steps:
    /// `Box::try?pin(x)` does the same as
    /// <code>[Box::into_pin]\([Box::try?new]\(x))</code>. Consider using
    /// [`into_pin`](Box::into_pin) if you already have a `Box<T>`, or if you
    /// want to construct a (pinned) `Box` in a different way than with
    /// [`Box::try_new`].
    #[inline(always)]
    pub fn try_pin(x: T) -> Result<Pin<Box<T>>, AllocError> {
        Ok(Box::try_new(x)?.into())
    }
}

impl<T: ?Sized> Box<T> {
    /// Convert from a std `Box`.
    ///
    /// This causes the underlying allocation to be accounted for by the
    /// [`Global`] allocator.
    ///
    /// A caveat of this method is that the allocation is already in use, but
    /// this might still be necessary because we want access to certain methods
    /// in std `Box` such as the ability to coerce to unsized values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{Box, Vec};
    /// use rune::alloc::limit;
    /// use std::boxed::Box as StdBox;
    ///
    /// assert_eq!(limit::get(), usize::MAX);
    ///
    /// let b: StdBox<dyn Iterator<Item = u32>> = StdBox::new(1..3);
    /// let mut b = Box::from_std(b)?;
    /// assert_eq!(b.next(), Some(1));
    /// assert_eq!(b.next(), Some(2));
    /// assert_eq!(b.next(), None);
    ///
    /// assert!(limit::get() < usize::MAX);
    /// drop(b);
    ///
    /// assert_eq!(limit::get(), usize::MAX);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn from_std(b: ::rust_alloc::boxed::Box<T>) -> Result<Self, Error> {
        // SAFETY: We've ensured that standard allocations only happen in an
        // allocator which is compatible with our `Global`.
        unsafe {
            // NB: Layout::for_value will return the size of the pointed to
            // value by the box, which for unsized types is the size of the
            // metadata. For sized types the value inside of the box.
            Global.take(Layout::for_value(b.as_ref()))?;
            let raw = ::rust_alloc::boxed::Box::into_raw(b);
            Ok(Box::from_raw_in(raw, Global))
        }
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
    ///
    /// let five = Box::try_new_in(5, Global)?;
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
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
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// # fn main() -> Result<(), rune::alloc::Error> {
    /// use rune::alloc::Box;
    ///
    /// let x = Box::try_new(41)?;
    /// let static_ref: &'static mut usize = Box::leak(x);
    /// *static_ref += 1;
    /// assert_eq!(*static_ref, 42);
    /// # Ok(())
    /// # }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    ///
    /// Unsized data:
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune::alloc::Error> {
    /// use rune::alloc::{try_vec, Box};
    ///
    /// let x = try_vec![1, 2, 3].try_into_boxed_slice()?;
    /// let static_ref = Box::leak(x);
    /// static_ref[0] = 4;
    /// assert_eq!(*static_ref, [4, 2, 3]);
    /// # Ok(())
    /// # }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    #[inline]
    pub fn leak<'a>(b: Self) -> &'a mut T
    where
        A: 'a,
    {
        unsafe { &mut *mem::ManuallyDrop::new(b).ptr.as_ptr() }
    }

    /// Converts a `Box<T>` into a `Pin<Box<T>>`. If `T` does not implement [`Unpin`], then
    /// `*boxed` will be pinned in memory and unable to be moved.
    ///
    /// This conversion does not allocate on the heap and happens in place.
    ///
    /// This is also available via [`From`].
    ///
    /// Constructing and pinning a `Box` with <code>Box::into_pin([Box::try?new]\(x))</code>
    /// can also be written more concisely using <code>[Box::try?pin]\(x)</code>.
    /// This `into_pin` method is useful if you already have a `Box<T>`, or you are
    /// constructing a (pinned) `Box` in a different way than with [`Box::try_new`].
    ///
    /// # Notes
    ///
    /// It's not recommended that crates add an impl like `From<Box<T>> for Pin<T>`,
    /// as it'll introduce an ambiguity when calling `Pin::from`.
    /// A demonstration of such a poor impl is shown below.
    ///
    /// ```compile_fail
    /// # use core::pin::Pin;
    /// use rune::alloc::Box;
    ///
    /// struct Foo; // A type defined in this crate.
    /// impl From<Box<()>> for Pin<Foo> {
    ///     fn from(_: Box<()>) -> Pin<Foo> {
    ///         Pin::new(Foo)
    ///     }
    /// }
    ///
    /// let foo = Box::try_new(())?;
    /// let bar = Pin::from(foo);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn into_pin(boxed: Self) -> Pin<Self>
    where
        A: 'static,
    {
        // It's not possible to move or replace the insides of a `Pin<Box<T>>`
        // when `T: !Unpin`, so it's safe to pin it directly without any
        // additional requirements.
        unsafe { Pin::new_unchecked(boxed) }
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
    ///
    /// let x = Box::try_new_in(5, Global)?;
    /// let (ptr, alloc) = Box::into_raw_with_allocator(x);
    /// let x = unsafe { Box::from_raw_in(ptr, alloc) };
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Manually create a `Box` from scratch by using the system allocator:
    ///
    /// ```
    /// use core::alloc::Layout;
    ///
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::{Allocator, Global};
    ///
    /// unsafe {
    ///     let ptr = Global.allocate(Layout::new::<i32>())?.as_ptr() as *mut i32;
    ///     // In general .write is required to avoid attempting to destruct
    ///     // the (uninitialized) previous contents of `ptr`, though for this
    ///     // simple example `*ptr = 5` would have worked as well.
    ///     ptr.write(5);
    ///     let x = Box::from_raw_in(ptr, Global);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::alloc::{Box, String};
    /// use rune::alloc::alloc::Global;
    ///
    /// let x = Box::try_new_in(String::try_from("Hello")?, Global)?;
    /// let (ptr, alloc) = Box::into_raw_with_allocator(x);
    /// let x = unsafe { Box::from_raw_in(ptr, alloc) };
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Manual cleanup by explicitly running the destructor and deallocating the
    /// memory:
    ///
    /// ```
    /// use core::alloc::Layout;
    /// use core::ptr::{self, NonNull};
    ///
    /// use rune::alloc::{Box, String};
    /// use rune::alloc::alloc::{Allocator, Global};
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
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
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
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
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
    /// # Ok::<_, rune::alloc::Error>(())
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
    /// use rune::alloc::Box;
    /// use rune::alloc::alloc::Global;
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
    /// # Ok::<_, rune::alloc::Error>(())
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
        let vec = crate::slice::to_vec(self, alloc)?;
        vec.try_into_boxed_slice()
    }
}

impl<A: Allocator + Clone> TryClone for Box<str, A> {
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        let alloc = self.alloc.clone();
        Box::try_from_string_in(self.as_ref(), alloc)
    }
}

impl<T: ?Sized, A: Allocator> Borrow<T> for Box<T, A> {
    fn borrow(&self) -> &T {
        self
    }
}

impl<T: ?Sized, A: Allocator> BorrowMut<T> for Box<T, A> {
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized, A: Allocator> AsRef<T> for Box<T, A> {
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized, A: Allocator> AsMut<T> for Box<T, A> {
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

/* Nota bene
 *
 *  We could have chosen not to add this impl, and instead have written a
 *  function of Pin<Box<T>> to Pin<T>. Such a function would not be sound,
 *  because Box<T> implements Unpin even when T does not, as a result of
 *  this impl.
 *
 *  We chose this API instead of the alternative for a few reasons:
 *      - Logically, it is helpful to understand pinning in regard to the
 *        memory region being pointed to. For this reason none of the
 *        standard library pointer types support projecting through a pin
 *        (Box<T> is the only pointer type in std for which this would be
 *        safe.)
 *      - It is in practice very useful to have Box<T> be unconditionally
 *        Unpin because of trait objects, for which the structural auto
 *        trait functionality does not apply (e.g., Box<dyn Foo> would
 *        otherwise not be Unpin).
 *
 *  Another type with the same semantics as Box but only a conditional
 *  implementation of `Unpin` (where `T: Unpin`) would be valid/safe, and
 *  could have a method to project a Pin<T> from it.
 */
impl<T: ?Sized, A: Allocator> Unpin for Box<T, A> where A: 'static {}

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

impl Default for Box<str, Global> {
    fn default() -> Self {
        // SAFETY: The layout of `Box<[u8]>` is the same as `Box<str>`.
        unsafe {
            let b = Box::<[u8]>::default();
            let (ptr, alloc) = Box::into_raw_with_allocator(b);
            Box::from_raw_in(ptr as *mut str, alloc)
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

impl<T, A: Allocator> TryFrom<Vec<T, A>> for Box<[T], A> {
    type Error = Error;

    #[inline]
    fn try_from(vec: Vec<T, A>) -> Result<Self, Error> {
        vec.try_into_boxed_slice()
    }
}

impl<A: Allocator> Box<[u8], A> {
    pub(crate) fn try_from_bytes_in(bytes: &[u8], alloc: A) -> Result<Self, Error> {
        let mut vec = Vec::<u8, A>::try_with_capacity_in(bytes.len(), alloc)?;

        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), vec.as_mut_ptr(), bytes.len());
            vec.set_len(bytes.len());
            vec.try_into_boxed_slice()
        }
    }
}

impl<A: Allocator> Box<str, A> {
    pub(crate) fn try_from_string_in(string: &str, alloc: A) -> Result<Self, Error> {
        unsafe {
            let b = Box::try_from_bytes_in(string.as_bytes(), alloc)?;
            let (raw, alloc) = Box::into_raw_with_allocator(b);
            Ok(Box::from_raw_in(raw as *mut str, alloc))
        }
    }
}

impl<A: Allocator> Box<Path, A> {
    pub(crate) fn try_from_path_in(path: &Path, alloc: A) -> Result<Self, Error> {
        unsafe {
            const _: () = assert!(mem::size_of::<&Path>() == mem::size_of::<&[u8]>());
            // Replace with path.as_os_str().as_encoded_bytes() once that is
            // stable.
            let bytes = &*(path as *const _ as *const [u8]);
            let b = Box::try_from_bytes_in(bytes, alloc)?;
            let (raw, alloc) = Box::into_raw_with_allocator(b);
            Ok(Box::from_raw_in(raw as *mut Path, alloc))
        }
    }
}

impl<A: Allocator + Clone> TryClone for Box<Path, A> {
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        let alloc = self.alloc.clone();
        Box::try_from_path_in(self.as_ref(), alloc)
    }
}

impl TryFrom<&str> for Box<str> {
    type Error = Error;

    /// Converts a `&str` into a `Box<str>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Box;
    ///
    /// let s: Box<str> = Box::try_from("Hello World")?;
    /// assert_eq!(s.as_ref(), "Hello World");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(values: &str) -> Result<Self, Error> {
        Box::try_from_string_in(values, Global)
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::string::String> for Box<str> {
    type Error = Error;

    /// Converts a std `String` into a `Box<str>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Box;
    ///
    /// let s = String::from("Hello World");
    /// let s: Box<str> = Box::try_from(s)?;
    /// assert_eq!(s.as_ref(), "Hello World");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(string: ::rust_alloc::string::String) -> Result<Self, Error> {
        Box::from_std(string.into_boxed_str())
    }
}

impl TryFrom<&[u8]> for Box<[u8]> {
    type Error = Error;

    /// Converts a `&[u8]` into a `Box<[u8]>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Box;
    ///
    /// let s: Box<[u8]> = Box::try_from(&b"Hello World"[..])?;
    /// assert_eq!(s.as_ref(), b"Hello World");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(values: &[u8]) -> Result<Self, Error> {
        Box::try_from_bytes_in(values, Global)
    }
}

impl TryFrom<&Path> for Box<Path> {
    type Error = Error;

    /// Converts a `&[u8]` into a `Box<[u8]>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use rune::alloc::Box;
    ///
    /// let path = Path::new("foo/bar");
    ///
    /// let s: Box<Path> = Box::try_from(path)?;
    /// assert_eq!(s.as_ref(), Path::new("foo/bar"));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(path: &Path) -> Result<Self, Error> {
        Box::try_from_path_in(path, Global)
    }
}

impl<T, A: Allocator> TryFromIteratorIn<T, A> for Box<[T], A> {
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        Vec::<T, A>::try_from_iter_in(iter, alloc)?.try_into_boxed_slice()
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

impl<T: ?Sized, A: Allocator> Hash for Box<T, A>
where
    T: Hash,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: ?Sized, A: Allocator> From<Box<T, A>> for Pin<Box<T, A>>
where
    A: 'static,
{
    /// Converts a `Box<T>` into a `Pin<Box<T>>`. If `T` does not implement
    /// [`Unpin`], then `*boxed` will be pinned in memory and unable to be
    /// moved.
    ///
    /// This conversion does not allocate on the heap and happens in place.
    ///
    /// This is also available via [`Box::into_pin`].
    ///
    /// Constructing and pinning a `Box` with
    /// <code><Pin<Box\<T>>>::from([Box::try?new]\(x))</code> can also be
    /// written more concisely using <code>[Box::try?pin]\(x)</code>. This
    /// `From` implementation is useful if you already have a `Box<T>`, or you
    /// are constructing a (pinned) `Box` in a different way than with
    /// [`Box::try_new`].
    fn from(boxed: Box<T, A>) -> Self {
        Box::into_pin(boxed)
    }
}

impl<T: ?Sized, A: Allocator> PartialEq for Box<T, A>
where
    T: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        (**self).eq(other)
    }
}

impl<T: ?Sized, A: Allocator> Eq for Box<T, A> where T: Eq {}

impl<T: ?Sized, A: Allocator> PartialOrd for Box<T, A>
where
    T: PartialOrd,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<T: ?Sized, A: Allocator> Ord for Box<T, A>
where
    T: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(other)
    }
}

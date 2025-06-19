//! Single-threaded reference-counting pointers. 'Rc' stands for 'Reference
//! Counted'.
//!
//! The type [`Rc<T>`][`Rc`] provides shared ownership of a value of type `T`,
//! allocated in the heap. Invoking [`clone`][clone] on [`Rc`] produces a new
//! pointer to the same allocation in the heap. When the last [`Rc`] pointer to a
//! given allocation is destroyed, the value stored in that allocation (often
//! referred to as "inner value") is also dropped.
//!
//! Shared references in Rust disallow mutation by default, and [`Rc`]
//! is no exception: you cannot generally obtain a mutable reference to
//! something inside an [`Rc`]. If you need mutability, put a [`Cell`]
//! or [`RefCell`] inside the [`Rc`]; see [an example of mutability
//! inside an `Rc`][mutability].
//!
//! [`Rc`] uses non-atomic reference counting. This means that overhead is very
//! low, but an [`Rc`] cannot be sent between threads, and consequently [`Rc`]
//! does not implement [`Send`]. As a result, the Rust compiler
//! will check *at compile time* that you are not sending [`Rc`]s between
//! threads. If you need multi-threaded, atomic reference counting, use
//! [`sync::Arc`][arc].
//!
//! The [`downgrade`][downgrade] method can be used to create a non-owning
//! [`Weak`] pointer. A [`Weak`] pointer can be [`upgrade`][upgrade]d
//! to an [`Rc`], but this will return [`None`] if the value stored in the allocation has
//! already been dropped. In other words, `Weak` pointers do not keep the value
//! inside the allocation alive; however, they *do* keep the allocation
//! (the backing store for the inner value) alive.
//!
//! A cycle between [`Rc`] pointers will never be deallocated. For this reason,
//! [`Weak`] is used to break cycles. For example, a tree could have strong
//! [`Rc`] pointers from parent nodes to children, and [`Weak`] pointers from
//! children back to their parents.
//!
//! `Rc<T>` automatically dereferences to `T` (via the [`Deref`] trait),
//! so you can call `T`'s methods on a value of type [`Rc<T>`][`Rc`]. To avoid name
//! clashes with `T`'s methods, the methods of [`Rc<T>`][`Rc`] itself are associated
//! functions, called using [fully qualified syntax]:
//!
//! ```
//! use rune::rc::Rc;
//!
//! let my_rc = Rc::try_new(())?;
//! let my_weak = Rc::downgrade(&my_rc);
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! `Rc<T>`'s implementations of traits like `Clone` may also be called using
//! fully qualified syntax. Some people prefer to use fully qualified syntax,
//! while others prefer using method-call syntax.
//!
//! ```
//! use rune::rc::Rc;
//!
//! let rc = Rc::try_new(())?;
//! // Method-call syntax
//! let rc2 = rc.clone();
//! // Fully qualified syntax
//! let rc3 = Rc::clone(&rc);
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! [`Weak<T>`][`Weak`] does not auto-dereference to `T`, because the inner value may have
//! already been dropped.
//!
//! # Cloning references
//!
//! Creating a new reference to the same allocation as an existing reference counted pointer
//! is done using the `Clone` trait implemented for [`Rc<T>`][`Rc`] and [`Weak<T>`][`Weak`].
//!
//! ```
//! use rune::rc::Rc;
//! use rune::alloc::prelude::*;
//!
//! let foo = Rc::try_new(try_vec![1.0, 2.0, 3.0])?;
//! // The two syntaxes below are equivalent.
//! let a = foo.clone();
//! let b = Rc::clone(&foo);
//! // a and b both point to the same memory location as foo.
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! The `Rc::clone(&from)` syntax is the most idiomatic because it conveys more explicitly
//! the meaning of the code. In the example above, this syntax makes it easier to see that
//! this code is creating a new reference rather than copying the whole content of foo.
//!
//! # Examples
//!
//! Consider a scenario where a set of `Gadget`s are owned by a given `Owner`.
//! We want to have our `Gadget`s point to their `Owner`. We can't do this with
//! unique ownership, because more than one gadget may belong to the same
//! `Owner`. [`Rc`] allows us to share an `Owner` between multiple `Gadget`s,
//! and have the `Owner` remain allocated as long as any `Gadget` points at it.
//!
//! ```
//! use rune::rc::Rc;
//! use rune::alloc;
//!
//! struct Owner {
//!     name: String,
//!     // ...other fields
//! }
//!
//! struct Gadget {
//!     id: i32,
//!     owner: Rc<Owner>,
//!     // ...other fields
//! }
//!
//! fn main() -> alloc::Result<()> {
//!     // Create a reference-counted `Owner`.
//!     let gadget_owner: Rc<Owner> = Rc::try_new(
//!         Owner {
//!             name: "Gadget Man".to_string(),
//!         }
//!     )?;
//!
//!     // Create `Gadget`s belonging to `gadget_owner`. Cloning the `Rc<Owner>`
//!     // gives us a new pointer to the same `Owner` allocation, incrementing
//!     // the reference count in the process.
//!     let gadget1 = Gadget {
//!         id: 1,
//!         owner: Rc::clone(&gadget_owner),
//!     };
//!     let gadget2 = Gadget {
//!         id: 2,
//!         owner: Rc::clone(&gadget_owner),
//!     };
//!
//!     // Dispose of our local variable `gadget_owner`.
//!     drop(gadget_owner);
//!
//!     // Despite dropping `gadget_owner`, we're still able to print out the name
//!     // of the `Owner` of the `Gadget`s. This is because we've only dropped a
//!     // single `Rc<Owner>`, not the `Owner` it points to. As long as there are
//!     // other `Rc<Owner>` pointing at the same `Owner` allocation, it will remain
//!     // live. The field projection `gadget1.owner.name` works because
//!     // `Rc<Owner>` automatically dereferences to `Owner`.
//!     println!("Gadget {} owned by {}", gadget1.id, gadget1.owner.name);
//!     println!("Gadget {} owned by {}", gadget2.id, gadget2.owner.name);
//!
//!     // At the end of the function, `gadget1` and `gadget2` are destroyed, and
//!     // with them the last counted references to our `Owner`. Gadget Man now
//!     // gets destroyed as well.
//!     Ok(())
//! }
//! ```
//!
//! If our requirements change, and we also need to be able to traverse from
//! `Owner` to `Gadget`, we will run into problems. An [`Rc`] pointer from `Owner`
//! to `Gadget` introduces a cycle. This means that their
//! reference counts can never reach 0, and the allocation will never be destroyed:
//! a memory leak. In order to get around this, we can use [`Weak`]
//! pointers.
//!
//! Rust actually makes it somewhat difficult to produce this loop in the first
//! place. In order to end up with two values that point at each other, one of
//! them needs to be mutable. This is difficult because [`Rc`] enforces
//! memory safety by only giving out shared references to the value it wraps,
//! and these don't allow direct mutation. We need to wrap the part of the
//! value we wish to mutate in a [`RefCell`], which provides *interior
//! mutability*: a method to achieve mutability through a shared reference.
//! [`RefCell`] enforces Rust's borrowing rules at runtime.
//!
//! ```
//! use std::cell::RefCell;
//!
//! use rune::rc::{Rc, Weak};
//!
//! struct Owner {
//!     name: String,
//!     gadgets: RefCell<Vec<Weak<Gadget>>>,
//!     // ...other fields
//! }
//!
//! struct Gadget {
//!     id: i32,
//!     owner: Rc<Owner>,
//!     // ...other fields
//! }
//!
//! fn main() {
//!     // Create a reference-counted `Owner`. Note that we've put the `Owner`'s
//!     // vector of `Gadget`s inside a `RefCell` so that we can mutate it through
//!     // a shared reference.
//!     let gadget_owner: Rc<Owner> = Rc::new(
//!         Owner {
//!             name: "Gadget Man".to_string(),
//!             gadgets: RefCell::new(vec![]),
//!         }
//!     );
//!
//!     // Create `Gadget`s belonging to `gadget_owner`, as before.
//!     let gadget1 = Rc::new(
//!         Gadget {
//!             id: 1,
//!             owner: Rc::clone(&gadget_owner),
//!         }
//!     );
//!     let gadget2 = Rc::new(
//!         Gadget {
//!             id: 2,
//!             owner: Rc::clone(&gadget_owner),
//!         }
//!     );
//!
//!     // Add the `Gadget`s to their `Owner`.
//!     {
//!         let mut gadgets = gadget_owner.gadgets.borrow_mut();
//!         gadgets.push(Rc::downgrade(&gadget1));
//!         gadgets.push(Rc::downgrade(&gadget2));
//!
//!         // `RefCell` dynamic borrow ends here.
//!     }
//!
//!     // Iterate over our `Gadget`s, printing their details out.
//!     for gadget_weak in gadget_owner.gadgets.borrow().iter() {
//!
//!         // `gadget_weak` is a `Weak<Gadget>`. Since `Weak` pointers can't
//!         // guarantee the allocation still exists, we need to call
//!         // `upgrade`, which returns an `Option<Rc<Gadget>>`.
//!         //
//!         // In this case we know the allocation still exists, so we simply
//!         // `unwrap` the `Option`. In a more complicated program, you might
//!         // need graceful error handling for a `None` result.
//!
//!         let gadget = gadget_weak.upgrade().unwrap();
//!         println!("Gadget {} owned by {}", gadget.id, gadget.owner.name);
//!     }
//!
//!     // At the end of the function, `gadget_owner`, `gadget1`, and `gadget2`
//!     // are destroyed. There are now no strong (`Rc`) pointers to the
//!     // gadgets, so they are destroyed. This zeroes the reference count on
//!     // Gadget Man, so he gets destroyed as well.
//! }
//! ```
//!
//! [clone]: Clone::clone
//! [`Cell`]: core::cell::Cell
//! [`RefCell`]: core::cell::RefCell
//! [arc]: crate::sync::Arc
//! [`Deref`]: core::ops::Deref
//! [downgrade]: Rc::downgrade
//! [upgrade]: Weak::upgrade
//! [mutability]: core::cell#introducing-mutability-inside-of-something-immutable
//! [fully qualified syntax]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name

use core::alloc::Layout;
use core::cell::Cell;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::ptr::{self, NonNull};

use crate::alloc::{AllocError, Allocator, Global};
use crate::{hint, Box};

// This is repr(C) to future-proof against possible field-reordering, which
// would interfere with otherwise safe [into|from]_raw() of transmutable
// inner types.
#[repr(C)]
struct RcInner<T: ?Sized> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

/// Calculate layout for `RcInner<T>` using the inner value's layout
fn rc_inner_layout_for_value_layout(layout: Layout) -> Layout {
    // Calculate layout using the given value layout.
    // Previously, layout was calculated on the expression
    // `&*(ptr as *const RcInner<T>)`, but this created a misaligned
    // reference (see #54908).
    Layout::new::<RcInner<()>>()
        .extend(layout)
        .unwrap()
        .0
        .pad_to_align()
}

/// A single-threaded reference-counting pointer. 'Rc' stands for 'Reference
/// Counted'.
///
/// See the [module-level documentation](./index.html) for more details.
///
/// The inherent methods of `Rc` are all associated functions, which means
/// that you have to call them as e.g., [`Rc::get_mut(&mut value)`][get_mut] instead of
/// `value.get_mut()`. This avoids conflicts with methods of the inner type `T`.
///
/// [get_mut]: Rc::get_mut
pub struct Rc<T, A = Global>
where
    T: ?Sized,
    A: Allocator,
{
    ptr: NonNull<RcInner<T>>,
    phantom: PhantomData<RcInner<T>>,
    alloc: A,
}

impl<T, A> UnwindSafe for Rc<T, A>
where
    T: RefUnwindSafe + ?Sized,
    A: Allocator + UnwindSafe,
{
}
impl<T, A> RefUnwindSafe for Rc<T, A>
where
    T: RefUnwindSafe + ?Sized,
    A: Allocator + UnwindSafe,
{
}

impl<T> Rc<T>
where
    T: ?Sized,
{
    #[inline]
    unsafe fn from_inner(ptr: NonNull<RcInner<T>>) -> Self {
        unsafe { Self::from_inner_in(ptr, Global) }
    }

    #[inline]
    unsafe fn from_ptr(ptr: *mut RcInner<T>) -> Self {
        unsafe { Self::from_inner(NonNull::new_unchecked(ptr)) }
    }
}

impl<T, A> Rc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    #[inline(always)]
    fn inner(&self) -> &RcInner<T> {
        // This unsafety is ok because while this Rc is alive we're guaranteed
        // that the inner pointer is valid.
        unsafe { self.ptr.as_ref() }
    }

    #[inline]
    fn into_inner_with_allocator(this: Self) -> (NonNull<RcInner<T>>, A) {
        let this = ManuallyDrop::new(this);
        (this.ptr, unsafe { ptr::read(&this.alloc) })
    }

    #[inline]
    unsafe fn from_inner_in(ptr: NonNull<RcInner<T>>, alloc: A) -> Self {
        Self {
            ptr,
            phantom: PhantomData,
            alloc,
        }
    }

    #[inline]
    unsafe fn from_ptr_in(ptr: *mut RcInner<T>, alloc: A) -> Self {
        unsafe { Self::from_inner_in(NonNull::new_unchecked(ptr), alloc) }
    }

    // Non-inlined part of `drop`.
    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        // Reconstruct the "strong weak" pointer and drop it when this
        // variable goes out of scope. This ensures that the memory is
        // deallocated even if the destructor of `T` panics.
        let _weak = Weak {
            ptr: self.ptr,
            alloc: &self.alloc,
        };

        // Destroy the contained object.
        // We cannot use `get_mut_unchecked` here, because `self.alloc` is borrowed.
        unsafe {
            ptr::drop_in_place(&mut (*self.ptr.as_ptr()).value);
        }
    }
}

impl<T> Rc<T> {
    /// Constructs a new `Rc<T>`, returning an error if the allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use rune::rc::Rc;
    ///
    /// let five = Rc::try_new(5);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_new(value: T) -> Result<Rc<T>, AllocError> {
        // There is an implicit weak pointer owned by all the strong
        // pointers, which ensures that the weak destructor never frees
        // the allocation while the strong destructor is running, even
        // if the weak pointer is stored inside the strong one.
        unsafe {
            let (ptr, alloc) = Box::into_unique_with_allocator(Box::try_new(RcInner {
                strong: Cell::new(1),
                weak: Cell::new(1),
                value,
            })?);

            Ok(Self::from_inner_in(ptr.into_nonnull(), alloc))
        }
    }
}

impl<T, A> Deref for Rc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.inner().value
    }
}

impl<T, A> Drop for Rc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Drops the `Rc`.
    ///
    /// This will decrement the strong reference count. If the strong reference
    /// count reaches zero then the only other references (if any) are
    /// [`Weak`], so we `drop` the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::Rc;
    ///
    /// struct Foo;
    ///
    /// impl Drop for Foo {
    ///     fn drop(&mut self) {
    ///         println!("dropped!");
    ///     }
    /// }
    ///
    /// let foo  = Rc::try_new(Foo)?;
    /// let foo2 = Rc::clone(&foo);
    ///
    /// drop(foo);    // Doesn't print anything
    /// drop(foo2);   // Prints "dropped!"
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn drop(&mut self) {
        unsafe {
            self.inner().dec_strong();

            if self.inner().strong() == 0 {
                self.drop_slow();
            }
        }
    }
}

impl<T, A> Clone for Rc<T, A>
where
    T: ?Sized,
    A: Allocator + Clone,
{
    /// Makes a clone of the `Rc` pointer.
    ///
    /// This creates another pointer to the same allocation, increasing the
    /// strong reference count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::Rc;
    ///
    /// let five = Rc::try_new(5)?;
    ///
    /// let _ = Rc::clone(&five);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            self.inner().inc_strong();
            Self::from_inner_in(self.ptr, self.alloc.clone())
        }
    }
}

/// Gets the offset within an `RcInner` for the payload behind a pointer.
///
/// # Safety
///
/// The pointer must point to (and have valid metadata for) a previously
/// valid instance of T, but the T is allowed to be dropped.
unsafe fn data_offset<T>(ptr: *const T) -> usize
where
    T: ?Sized,
{
    // Align the unsized value to the end of the RcInner.
    // Because RcInner is repr(C), it will always be the last field in memory.
    // SAFETY: since the only unsized types possible are slices, trait objects,
    // and extern types, the input safety requirement is currently enough to
    // satisfy the requirements of align_of_val_raw; this is an implementation
    // detail of the language that must not be relied upon outside of std.
    unsafe { data_offset_align(align_of_val(&*ptr)) }
}

#[inline]
fn data_offset_align(align: usize) -> usize {
    let layout = Layout::new::<RcInner<()>>();
    layout.size() + padding_needed_for(&layout, align)
}

const fn padding_needed_for(this: &Layout, align: usize) -> usize {
    // TODO: Switch to `Alignment` once stable.
    let align = if align.is_power_of_two() {
        align
    } else {
        return usize::MAX;
    };
    let len_rounded_up = size_rounded_up_to_custom_align(this, align);
    // SAFETY: Cannot overflow because the rounded-up value is never less
    unsafe { len_rounded_up.unchecked_sub(this.size()) }
}

/// Returns the smallest multiple of `align` greater than or equal to
/// `self.size()`.
///
/// This can return at most `Alignment::MAX` (aka `isize::MAX + 1`) because the
/// original size is at most `isize::MAX`.
#[inline]
const fn size_rounded_up_to_custom_align(layout: &Layout, align: usize) -> usize {
    // SAFETY: Rounded up value is: size_rounded_up = (size + align - 1) &
    // !(align - 1);
    //
    // The arithmetic we do here can never overflow:
    //
    // 1. align is guaranteed to be > 0, so align - 1 is always valid.
    //
    // 2. size is at most `isize::MAX`, so adding `align - 1` (which is at most
    //    `isize::MAX`) can never overflow a `usize`.
    //
    // 3. masking by the alignment can remove at most `align - 1`, which is what
    //    we just added, thus the value we return is never less than the
    //    original `size`.
    //
    // (Size 0 Align MAX is already aligned, so stays the same, but things like
    // Size 1 Align MAX or Size isize::MAX Align 2 round up to `isize::MAX +
    // 1`.)
    unsafe {
        let align_m1 = align.unchecked_sub(1);
        layout.size().unchecked_add(align_m1) & !align_m1
    }
}

/// `Weak` is a version of [`Rc`] that holds a non-owning reference to the
/// managed allocation.
///
/// The allocation is accessed by calling [`upgrade`] on the `Weak`
/// pointer, which returns an <code>[Option]<[Rc]\<T>></code>.
///
/// Since a `Weak` reference does not count towards ownership, it will not
/// prevent the value stored in the allocation from being dropped, and `Weak` itself makes no
/// guarantees about the value still being present. Thus it may return [`None`]
/// when [`upgrade`]d. Note however that a `Weak` reference *does* prevent the allocation
/// itself (the backing store) from being deallocated.
///
/// A `Weak` pointer is useful for keeping a temporary reference to the allocation
/// managed by [`Rc`] without preventing its inner value from being dropped. It is also used to
/// prevent circular references between [`Rc`] pointers, since mutual owning references
/// would never allow either [`Rc`] to be dropped. For example, a tree could
/// have strong [`Rc`] pointers from parent nodes to children, and `Weak`
/// pointers from children back to their parents.
///
/// The typical way to obtain a `Weak` pointer is to call [`Rc::downgrade`].
///
/// [`upgrade`]: Weak::upgrade
pub struct Weak<T, A = Global>
where
    T: ?Sized,
    A: Allocator,
{
    // This is a `NonNull` to allow optimizing the size of this type in enums,
    // but it is not necessarily a valid pointer.
    // `Weak::new` sets this to `usize::MAX` so that it doesn’t need
    // to allocate space on the heap. That's not a value a real pointer
    // will ever have because RcInner has alignment at least 2.
    // This is only possible when `T: Sized`; unsized `T` never dangle.
    ptr: NonNull<RcInner<T>>,
    alloc: A,
}

impl<T, A> Weak<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Returns a reference to the underlying allocator.
    #[inline]
    pub fn allocator(&self) -> &A {
        &self.alloc
    }

    /// Returns a raw pointer to the object `T` pointed to by this `Weak<T>`.
    ///
    /// The pointer is valid only if there are some strong references. The pointer may be dangling,
    /// unaligned or even [`null`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::Rc;
    /// use rune::alloc::prelude::*;
    ///
    /// use std::ptr;
    ///
    /// let strong = Rc::try_new("hello".try_to_owned()?)?;
    /// let weak = Rc::downgrade(&strong);
    /// // Both point to the same object
    /// assert!(ptr::eq(&*strong, weak.as_ptr()));
    /// // The strong here keeps it alive, so we can still access the object.
    /// assert_eq!("hello", unsafe { &*weak.as_ptr() });
    ///
    /// drop(strong);
    /// // But not any more. We can do weak.as_ptr(), but accessing the pointer would lead to
    /// // undefined behavior.
    /// // assert_eq!("hello", unsafe { &*weak.as_ptr() });
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`null`]: ptr::null
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        let ptr: *mut RcInner<T> = NonNull::as_ptr(self.ptr);

        if is_dangling(ptr) {
            // If the pointer is dangling, we return the sentinel directly. This cannot be
            // a valid payload address, as the payload is at least as aligned as RcInner (usize).
            ptr as *const T
        } else {
            // SAFETY: if is_dangling returns false, then the pointer is dereferenceable.
            // The payload may be dropped at this point, and we have to maintain provenance,
            // so use raw pointer manipulation.
            unsafe { &raw mut (*ptr).value }
        }
    }

    /// Consumes the `Weak<T>` and turns it into a raw pointer.
    ///
    /// This converts the weak pointer into a raw pointer, while still preserving the ownership of
    /// one weak reference (the weak count is not modified by this operation). It can be turned
    /// back into the `Weak<T>` with [`from_raw`].
    ///
    /// The same restrictions of accessing the target of the pointer as with
    /// [`as_ptr`] apply.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::{Rc, Weak};
    ///
    /// let strong = Rc::try_new("hello".to_owned())?;
    /// let weak = Rc::downgrade(&strong);
    /// let raw = weak.into_raw();
    ///
    /// assert_eq!(1, Rc::weak_count(&strong));
    /// assert_eq!("hello", unsafe { &*raw });
    ///
    /// drop(unsafe { Weak::from_raw(raw) });
    /// assert_eq!(0, Rc::weak_count(&strong));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`from_raw`]: Weak::from_raw
    /// [`as_ptr`]: Weak::as_ptr
    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw(self) -> *const T {
        ManuallyDrop::new(self).as_ptr()
    }

    /// Consumes the `Weak<T>`, returning the wrapped pointer and allocator.
    ///
    /// This converts the weak pointer into a raw pointer, while still preserving the ownership of
    /// one weak reference (the weak count is not modified by this operation). It can be turned
    /// back into the `Weak<T>` with [`from_raw_in`].
    ///
    /// The same restrictions of accessing the target of the pointer as with
    /// [`as_ptr`] apply.
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(allocator_api)]
    /// use rune::rc::{Rc, Weak};
    /// use std::alloc::System;
    ///
    /// let strong = Rc::new_in("hello".to_owned(), System);
    /// let weak = Rc::downgrade(&strong);
    /// let (raw, alloc) = weak.into_raw_with_allocator();
    ///
    /// assert_eq!(1, Rc::weak_count(&strong));
    /// assert_eq!("hello", unsafe { &*raw });
    ///
    /// drop(unsafe { Weak::from_raw_in(raw, alloc) });
    /// assert_eq!(0, Rc::weak_count(&strong));
    /// ```
    ///
    /// [`from_raw_in`]: Weak::from_raw_in
    /// [`as_ptr`]: Weak::as_ptr
    #[must_use = "losing the pointer will leak memory"]
    #[inline]
    pub fn into_raw_with_allocator(self) -> (*const T, A) {
        let this = ManuallyDrop::new(self);
        let result = this.as_ptr();
        // Safety: `this` is ManuallyDrop so the allocator will not be double-dropped
        let alloc = unsafe { ptr::read(&this.alloc) };
        (result, alloc)
    }

    /// Converts a raw pointer previously created by [`into_raw`] back into `Weak<T>`.
    ///
    /// This can be used to safely get a strong reference (by calling [`upgrade`]
    /// later) or to deallocate the weak count by dropping the `Weak<T>`.
    ///
    /// It takes ownership of one weak reference (with the exception of pointers created by [`new`],
    /// as these don't own anything; the method still works on them).
    ///
    /// # Safety
    ///
    /// The pointer must have originated from the [`into_raw`] and must still own its potential
    /// weak reference, and `ptr` must point to a block of memory allocated by `alloc`.
    ///
    /// It is allowed for the strong count to be 0 at the time of calling this. Nevertheless, this
    /// takes ownership of one weak reference currently represented as a raw pointer (the weak
    /// count is not modified by this operation) and therefore it must be paired with a previous
    /// call to [`into_raw`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::{Rc, Weak};
    /// use rune::alloc::prelude::*;
    ///
    /// let strong = Rc::try_new("hello".try_to_owned()?)?;
    ///
    /// let raw_1 = Rc::downgrade(&strong).into_raw();
    /// let raw_2 = Rc::downgrade(&strong).into_raw();
    ///
    /// assert_eq!(2, Rc::weak_count(&strong));
    ///
    /// assert_eq!("hello", &*unsafe { Weak::from_raw(raw_1) }.upgrade().unwrap());
    /// assert_eq!(1, Rc::weak_count(&strong));
    ///
    /// drop(strong);
    ///
    /// // Decrement the last weak count.
    /// assert!(unsafe { Weak::from_raw(raw_2) }.upgrade().is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`into_raw`]: Weak::into_raw
    /// [`upgrade`]: Weak::upgrade
    /// [`new`]: Weak::new
    #[inline]
    pub unsafe fn from_raw_in(ptr: *const T, alloc: A) -> Self {
        // See Weak::as_ptr for context on how the input pointer is derived.

        let ptr = if is_dangling(ptr) {
            // This is a dangling Weak.
            ptr as *mut RcInner<T>
        } else {
            // Otherwise, we're guaranteed the pointer came from a nondangling Weak.
            // SAFETY: data_offset is safe to call, as ptr references a real (potentially dropped) T.
            let offset = unsafe { data_offset(ptr) };
            // Thus, we reverse the offset to get the whole RcInner.
            // SAFETY: the pointer originated from a Weak, so this offset is safe.
            unsafe { ptr.byte_sub(offset) as *mut RcInner<T> }
        };

        // SAFETY: we now have recovered the original Weak pointer, so can create the Weak.
        Weak {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            alloc,
        }
    }

    /// Attempts to upgrade the `Weak` pointer to an [`Rc`], delaying
    /// dropping of the inner value if successful.
    ///
    /// Returns [`None`] if the inner value has since been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::Rc;
    ///
    /// let five = Rc::try_new(5)?;
    ///
    /// let weak_five = Rc::downgrade(&five);
    ///
    /// let strong_five: Option<Rc<_>> = weak_five.upgrade();
    /// assert!(strong_five.is_some());
    ///
    /// // Destroy all strong pointers.
    /// drop(strong_five);
    /// drop(five);
    ///
    /// assert!(weak_five.upgrade().is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "this returns a new `Rc`, \
                  without modifying the original weak pointer"]
    pub fn upgrade(&self) -> Option<Rc<T, A>>
    where
        A: Clone,
    {
        let inner = self.inner()?;

        if inner.strong() == 0 {
            None
        } else {
            unsafe {
                inner.inc_strong();
                Some(Rc::from_inner_in(self.ptr, self.alloc.clone()))
            }
        }
    }

    /// Gets the number of strong (`Rc`) pointers pointing to this allocation.
    ///
    /// If `self` was created using [`Weak::new`], this will return 0.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        if let Some(inner) = self.inner() {
            inner.strong()
        } else {
            0
        }
    }

    /// Gets the number of `Weak` pointers pointing to this allocation.
    ///
    /// If no strong pointers remain, this will return zero.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        if let Some(inner) = self.inner() {
            if inner.strong() > 0 {
                inner.weak() - 1 // subtract the implicit weak ptr
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Returns `None` when the pointer is dangling and there is no allocated `RcInner`,
    /// (i.e., when this `Weak` was created by `Weak::new`).
    #[inline]
    fn inner(&self) -> Option<WeakInner<'_>> {
        if is_dangling(self.ptr.as_ptr()) {
            None
        } else {
            // We are careful to *not* create a reference covering the "data" field, as
            // the field may be mutated concurrently (for example, if the last `Rc`
            // is dropped, the data field will be dropped in-place).
            Some(unsafe {
                let ptr = self.ptr.as_ptr();
                WeakInner {
                    strong: &(*ptr).strong,
                    weak: &(*ptr).weak,
                }
            })
        }
    }

    /// Returns `true` if the two `Weak`s point to the same allocation similar to [`ptr::eq`], or if
    /// both don't point to any allocation (because they were created with `Weak::new()`). However,
    /// this function ignores the metadata of  `dyn Trait` pointers.
    ///
    /// # Notes
    ///
    /// Since this compares pointers it means that `Weak::new()` will equal each
    /// other, even though they don't point to any allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::Rc;
    ///
    /// let first_rc = Rc::try_new(5)?;
    /// let first = Rc::downgrade(&first_rc);
    /// let second = Rc::downgrade(&first_rc);
    ///
    /// assert!(first.ptr_eq(&second));
    ///
    /// let third_rc = Rc::try_new(5)?;
    /// let third = Rc::downgrade(&third_rc);
    ///
    /// assert!(!first.ptr_eq(&third));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Comparing `Weak::new`.
    ///
    /// ```
    /// use rune::rc::{Rc, Weak};
    ///
    /// let first = Weak::new();
    /// let second = Weak::new();
    /// assert!(first.ptr_eq(&second));
    ///
    /// let third_rc = Rc::try_new(())?;
    /// let third = Rc::downgrade(&third_rc);
    /// assert!(!first.ptr_eq(&third));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self.ptr.as_ptr(), other.ptr.as_ptr())
    }
}

impl<T, A> Drop for Weak<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Drops the `Weak` pointer.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::rc::{Rc, Weak};
    ///
    /// struct Foo;
    ///
    /// impl Drop for Foo {
    ///     fn drop(&mut self) {
    ///         println!("dropped!");
    ///     }
    /// }
    ///
    /// let foo = Rc::try_new(Foo)?;
    /// let weak_foo = Rc::downgrade(&foo);
    /// let other_weak_foo = Weak::clone(&weak_foo);
    ///
    /// drop(weak_foo);   // Doesn't print anything
    /// drop(foo);        // Prints "dropped!"
    ///
    /// assert!(other_weak_foo.upgrade().is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn drop(&mut self) {
        let inner = if let Some(inner) = self.inner() {
            inner
        } else {
            return;
        };

        inner.dec_weak();
        // the weak count starts at 1, and will only go to zero if all
        // the strong pointers have disappeared.
        if inner.weak() == 0 {
            unsafe {
                // TODO: Replace with `Layout::for_value_raw` once stabilized.
                let layout = Layout::for_value(&*self.ptr.as_ptr());
                self.alloc.deallocate(self.ptr.cast(), layout);
            }
        }
    }
}

pub(crate) fn is_dangling<T: ?Sized>(ptr: *const T) -> bool {
    (ptr.cast::<()>()).addr() == usize::MAX
}

/// Helper type to allow accessing the reference counts without
/// making any assertions about the data field.
struct WeakInner<'a> {
    weak: &'a Cell<usize>,
    strong: &'a Cell<usize>,
}

#[doc(hidden)]
trait RcInnerPtr {
    fn weak_ref(&self) -> &Cell<usize>;
    fn strong_ref(&self) -> &Cell<usize>;

    #[inline]
    fn strong(&self) -> usize {
        self.strong_ref().get()
    }

    #[inline]
    fn inc_strong(&self) {
        let strong = self.strong();

        // We insert an `assume` here to hint LLVM at an otherwise
        // missed optimization.
        // SAFETY: The reference count will never be zero when this is
        // called.
        unsafe {
            hint::assert_unchecked(strong != 0);
        }

        let strong = strong.wrapping_add(1);
        self.strong_ref().set(strong);

        // We want to abort on overflow instead of dropping the value.
        // Checking for overflow after the store instead of before
        // allows for slightly better code generation.
        if hint::unlikely(strong == 0) {
            crate::abort();
        }
    }

    #[inline]
    fn dec_strong(&self) {
        self.strong_ref().set(self.strong() - 1);
    }

    #[inline]
    fn weak(&self) -> usize {
        self.weak_ref().get()
    }

    #[inline]
    fn inc_weak(&self) {
        let weak = self.weak();

        // We insert an `assume` here to hint LLVM at an otherwise
        // missed optimization.
        // SAFETY: The reference count will never be zero when this is
        // called.
        unsafe {
            hint::assert_unchecked(weak != 0);
        }

        let weak = weak.wrapping_add(1);
        self.weak_ref().set(weak);

        // We want to abort on overflow instead of dropping the value.
        // Checking for overflow after the store instead of before
        // allows for slightly better code generation.
        if hint::unlikely(weak == 0) {
            crate::abort();
        }
    }

    #[inline]
    fn dec_weak(&self) {
        self.weak_ref().set(self.weak() - 1);
    }
}

impl<T> RcInnerPtr for RcInner<T>
where
    T: ?Sized,
{
    #[inline(always)]
    fn weak_ref(&self) -> &Cell<usize> {
        &self.weak
    }

    #[inline(always)]
    fn strong_ref(&self) -> &Cell<usize> {
        &self.strong
    }
}

impl RcInnerPtr for WeakInner<'_> {
    #[inline(always)]
    fn weak_ref(&self) -> &Cell<usize> {
        self.weak
    }

    #[inline(always)]
    fn strong_ref(&self) -> &Cell<usize> {
        self.strong
    }
}

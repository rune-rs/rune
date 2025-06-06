use core::alloc::Layout;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::hint;
use core::marker::PhantomData;
use core::mem;
use core::ops::Deref;
use core::panic::RefUnwindSafe;
use core::panic::UnwindSafe;
use core::ptr;
use core::ptr::NonNull;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

use crate::alloc::{AllocError, Allocator, Global};
use crate::clone::TryClone;
use crate::{abort, Box, Result, Vec};

fn is_dangling<T: ?Sized>(ptr: *const T) -> bool {
    (ptr.cast::<()>()).addr() == usize::MAX
}

#[must_use]
unsafe fn for_value_raw<T>(t: *const T) -> Layout
where
    T: ?Sized,
{
    // SAFETY: we pass along the prerequisites of these functions to the caller
    // TODO: Use mem::{size_of_val_raw, align_of_val_raw} when they become
    // stable, for now we privately know that this can safely be turned into a
    // reference since it's only used while dropping an owned value of type `T`.
    let (size, align) = unsafe { (mem::size_of_val(&*t), mem::align_of_val(&*t)) };
    // SAFETY: see rationale in `new` for why this is using the unsafe variant
    unsafe { Layout::from_size_align_unchecked(size, align) }
}

/// A soft limit on the amount of references that may be made to an `Arc`.
///
/// Going above this limit will abort your program (although not necessarily) at
/// _exactly_ `MAX_REFCOUNT + 1` references. Trying to go above it might call a
/// `panic` (if not actually going above it).
///
/// This is a global invariant, and also applies when using a compare-exchange
/// loop.
///
/// See comment in `Arc::clone`.
const MAX_REFCOUNT: usize = (isize::MAX) as usize;

/// The error in case either counter reaches above `MAX_REFCOUNT`, and we can `panic` safely.
const INTERNAL_OVERFLOW_ERROR: &str = "Arc counter overflow";

macro_rules! acquire {
    ($x:expr) => {
        core::sync::atomic::fence(Acquire)
    };
}

/// A thread-safe reference-counting pointer. 'Arc' stands for 'Atomically
/// Reference Counted'.
///
/// The type `Arc<T>` provides shared ownership of a value of type `T`,
/// allocated in the heap. Invoking [`clone`][clone] on `Arc` produces a new
/// `Arc` instance, which points to the same allocation on the heap as the
/// source `Arc`, while increasing a reference count. When the last `Arc`
/// pointer to a given allocation is destroyed, the value stored in that
/// allocation (often referred to as "inner value") is also dropped.
///
/// Shared references in Rust disallow mutation by default, and `Arc` is no
/// exception: you cannot generally obtain a mutable reference to something
/// inside an `Arc`. If you do need to mutate through an `Arc`, you have several
/// options:
///
/// 1. Use interior mutability with synchronization primitives like
///    [`Mutex`][mutex], [`RwLock`][rwlock], or one of the [`Atomic`][atomic]
///    types.
///
/// 2. Use [`Arc::get_mut`] when you know your `Arc` is not shared (has a
///    reference count of 1), which provides direct mutable access to the inner
///    value without any cloning.
///
/// ```
/// use rune::sync::Arc;
/// use rune::alloc::try_vec;
///
/// let mut data = Arc::try_new(try_vec![1, 2, 3])?;
///
/// // This will clone the vector only if there are other references to it
/// Arc::get_mut(&mut data).unwrap().try_push(4)?;
///
/// assert_eq!(*data, try_vec![1, 2, 3, 4]);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// **Note**: This type is only available on platforms that support atomic loads
/// and stores of pointers, which includes all platforms that support the `std`
/// crate but not all those which only support [`alloc`](crate). This may be
/// detected at compile time using `#[cfg(target_has_atomic = "ptr")]`.
///
/// ## Thread Safety
///
/// Unlike `Rc<T>`, `Arc<T>` uses atomic operations for its reference counting.
/// This means that it is thread-safe. The disadvantage is that atomic
/// operations are more expensive than ordinary memory accesses. If you are not
/// sharing reference-counted allocations between threads, consider using
/// `Rc<T>` for lower overhead. `Rc<T>` is a safe default, because the compiler
/// will catch any attempt to send an `Rc<T>` between threads. However, a
/// library might choose `Arc<T>` in order to give library consumers more
/// flexibility.
///
/// `Arc<T>` will implement [`Send`] and [`Sync`] as long as the `T` implements
/// [`Send`] and [`Sync`]. Why can't you put a non-thread-safe type `T` in an
/// `Arc<T>` to make it thread-safe? This may be a bit counter-intuitive at
/// first: after all, isn't the point of `Arc<T>` thread safety? The key is
/// this: `Arc<T>` makes it thread safe to have multiple ownership of the same
/// data, but it  doesn't add thread safety to its data. Consider
/// <code>Arc<[RefCell\<T>]></code>. [`RefCell<T>`] isn't [`Sync`], and if
/// `Arc<T>` was always [`Send`], <code>Arc<[RefCell\<T>]></code> would be as
/// well. But then we'd have a problem: [`RefCell<T>`] is not thread safe; it
/// keeps track of the borrowing count using non-atomic operations.
///
/// In the end, this means that you may need to pair `Arc<T>` with some sort of
/// [`std::sync`] type, usually [`Mutex<T>`][mutex].
///
/// ## Breaking cycles with `Weak`
///
/// The [`downgrade`][downgrade] method can be used to create a non-owning
/// [`Weak`] pointer. A [`Weak`] pointer can be [`upgrade`][upgrade]d to an
/// `Arc`, but this will return [`None`] if the value stored in the allocation
/// has already been dropped. In other words, `Weak` pointers do not keep the
/// value inside the allocation alive; however, they *do* keep the allocation
/// (the backing store for the value) alive.
///
/// A cycle between `Arc` pointers will never be deallocated. For this reason,
/// [`Weak`] is used to break cycles. For example, a tree could have strong
/// `Arc` pointers from parent nodes to children, and [`Weak`] pointers from
/// children back to their parents.
///
/// # Cloning references
///
/// Creating a new reference from an existing reference-counted pointer is done
/// using the `Clone` trait implemented for [`Arc<T>`][Arc] and
/// [`Weak<T>`][Weak].
///
/// ```
/// use rune::sync::Arc;
/// use rune::alloc::try_vec;
///
/// let foo = Arc::try_new(try_vec![1.0, 2.0, 3.0])?;
/// // The two syntaxes below are equivalent.
/// let a = foo.clone();
/// let b = Arc::clone(&foo);
/// // a, b, and foo are all Arcs that point to the same memory location
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// ## `Deref` behavior
///
/// `Arc<T>` automatically dereferences to `T` (via the [`Deref`] trait), so you
/// can call `T`'s methods on a value of type `Arc<T>`. To avoid name clashes
/// with `T`'s methods, the methods of `Arc<T>` itself are associated functions,
/// called using [fully qualified syntax]:
///
/// ```
/// use rune::sync::Arc;
///
/// let my_arc = Arc::try_new(())?;
/// let my_weak = Arc::downgrade(&my_arc);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// `Arc<T>`'s implementations of traits like `Clone` may also be called using
/// fully qualified syntax. Some people prefer to use fully qualified syntax,
/// while others prefer using method-call syntax.
///
/// ```
/// use rune::sync::Arc;
///
/// let arc = Arc::try_new(())?;
/// // Method-call syntax
/// let arc2 = arc.clone();
/// // Fully qualified syntax
/// let arc3 = Arc::clone(&arc);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`Weak<T>`][Weak] does not auto-dereference to `T`, because the inner value
/// may have already been dropped.
///
/// [clone]: Clone::clone
/// [mutex]: ../../std/sync/struct.Mutex.html
/// [rwlock]: ../../std/sync/struct.RwLock.html
/// [atomic]: core::sync::atomic
/// [downgrade]: Arc::downgrade
/// [upgrade]: Weak::upgrade
/// [RefCell\<T>]: core::cell::RefCell
/// [`RefCell<T>`]: core::cell::RefCell
/// [`std::sync`]: ../../std/sync/index.html
/// [`Arc::clone(&from)`]: Arc::clone
/// [fully qualified syntax]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#fully-qualified-syntax-for-disambiguation-calling-methods-with-the-same-name
///
/// # Examples
///
/// Sharing some immutable data between threads:
///
/// ```
/// use std::thread;
///
/// use rune::sync::Arc;
///
/// let five = Arc::try_new(5)?;
///
/// for _ in 0..10 {
///     let five = Arc::clone(&five);
///
///     thread::spawn(move || {
///         println!("{five:?}");
///     });
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Sharing a mutable [`AtomicUsize`]:
///
/// [`AtomicUsize`]: core::sync::atomic::AtomicUsize "sync::atomic::AtomicUsize"
///
/// ```
/// use std::sync::atomic::{AtomicUsize, Ordering};
/// use std::thread;
///
/// use rune::sync::Arc;
///
/// let val = Arc::try_new(AtomicUsize::new(5))?;
///
/// for _ in 0..10 {
///     let val = Arc::clone(&val);
///
///     thread::spawn(move || {
///         let v = val.fetch_add(1, Ordering::Relaxed);
///         println!("{v:?}");
///     });
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub struct Arc<T, A = Global>
where
    T: ?Sized,
    A: Allocator,
{
    ptr: NonNull<ArcInner<T>>,
    phantom: PhantomData<ArcInner<T>>,
    alloc: A,
}

unsafe impl<T, A> Send for Arc<T, A>
where
    T: ?Sized + Sync + Send,
    A: Allocator + Send,
{
}

unsafe impl<T, A> Sync for Arc<T, A>
where
    T: ?Sized + Sync + Send,
    A: Allocator + Sync,
{
}

impl<T, A> UnwindSafe for Arc<T, A>
where
    T: RefUnwindSafe + ?Sized,
    A: Allocator + UnwindSafe,
{
}

impl<T> Arc<[T]> {
    /// Copy elements from slice into newly allocated `Arc<[T]>`
    #[doc(hidden)]
    pub fn copy_from_slice(v: &[T]) -> Result<Self, AllocError>
    where
        T: Copy,
    {
        Self::copy_from_slice_in(v, Global)
    }
}

impl<T, A> Arc<[T], A>
where
    A: Allocator,
{
    /// Allocates an `ArcInner<[T]>` with the given length.
    unsafe fn try_allocate_for_slice_in(
        len: usize,
        alloc: &A,
    ) -> Result<*mut ArcInner<[T]>, AllocError> {
        unsafe {
            Self::try_allocate_for_layout(
                Layout::array::<T>(len).unwrap(),
                |layout| alloc.allocate(layout),
                |mem| ptr::slice_from_raw_parts_mut(mem.cast::<T>(), len) as *mut ArcInner<[T]>,
            )
        }
    }

    /// Copy elements from slice into newly allocated `Arc<[T]>`
    #[doc(hidden)]
    pub fn copy_from_slice_in(v: &[T], alloc: A) -> Result<Self, AllocError>
    where
        T: Copy,
    {
        unsafe {
            let ptr = Self::try_allocate_for_slice_in(v.len(), &alloc)?;
            ptr::copy_nonoverlapping(v.as_ptr(), (&raw mut (*ptr).data) as *mut T, v.len());
            Ok(Self::from_ptr_in(ptr, alloc))
        }
    }
}

/// `Weak` is a version of [`Arc`] that holds a non-owning reference to the
/// managed allocation.
///
/// The allocation is accessed by calling [`upgrade`] on the `Weak`
/// pointer, which returns an <code>[Option]<[Arc]\<T>></code>.
///
/// Since a `Weak` reference does not count towards ownership, it will not
/// prevent the value stored in the allocation from being dropped, and `Weak` itself makes no
/// guarantees about the value still being present. Thus it may return [`None`]
/// when [`upgrade`]d. Note however that a `Weak` reference *does* prevent the allocation
/// itself (the backing store) from being deallocated.
///
/// A `Weak` pointer is useful for keeping a temporary reference to the allocation
/// managed by [`Arc`] without preventing its inner value from being dropped. It is also used to
/// prevent circular references between [`Arc`] pointers, since mutual owning references
/// would never allow either [`Arc`] to be dropped. For example, a tree could
/// have strong [`Arc`] pointers from parent nodes to children, and `Weak`
/// pointers from children back to their parents.
///
/// The typical way to obtain a `Weak` pointer is to call [`Arc::downgrade`].
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
    ptr: NonNull<ArcInner<T>>,
    alloc: A,
}

/// Helper type to allow accessing the reference counts without making any
/// assertions about the data field.
struct WeakInner<'a> {
    weak: &'a AtomicUsize,
    #[allow(unused)]
    strong: &'a AtomicUsize,
}

impl<T, A> Weak<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Attempts to upgrade the `Weak` pointer to an [`Arc`], delaying dropping
    /// of the inner value if successful.
    ///
    /// Returns [`None`] if the inner value has since been dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// let weak_five = Arc::downgrade(&five);
    ///
    /// let strong_five: Option<Arc<_>> = weak_five.upgrade();
    /// assert!(strong_five.is_some());
    ///
    /// // Destroy all strong pointers.
    /// drop(strong_five);
    /// drop(five);
    ///
    /// assert!(weak_five.upgrade().is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "this returns a new `Arc`, \
                  without modifying the original weak pointer"]
    pub fn upgrade(&self) -> Option<Arc<T, A>>
    where
        A: Clone,
    {
        #[inline]
        fn checked_increment(n: usize) -> Option<usize> {
            // Any write of 0 we can observe leaves the field in permanently zero state.
            if n == 0 {
                return None;
            }

            // See comments in `Arc::clone` for why we do this (for `mem::forget`).
            assert!(n <= MAX_REFCOUNT, "{}", INTERNAL_OVERFLOW_ERROR);
            Some(n + 1)
        }

        // We use a CAS loop to increment the strong count instead of a
        // fetch_add as this function should never take the reference count
        // from zero to one.
        //
        // Relaxed is fine for the failure case because we don't have any expectations about the new state.
        // Acquire is necessary for the success case to synchronise with `Arc::new_cyclic`, when the inner
        // value can be initialized after `Weak` references have already been created. In that case, we
        // expect to observe the fully initialized value.
        if self
            .inner()?
            .strong
            .fetch_update(Acquire, Relaxed, checked_increment)
            .is_ok()
        {
            // SAFETY: pointer is not null, verified in checked_increment
            unsafe { Some(Arc::from_inner_in(self.ptr, self.alloc.clone())) }
        } else {
            None
        }
    }

    /// Gets the number of strong (`Arc`) pointers pointing to this allocation.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        if let Some(inner) = self.inner() {
            inner.strong.load(Relaxed)
        } else {
            0
        }
    }

    /// Gets an approximation of the number of `Weak` pointers pointing to this
    /// allocation.
    ///
    /// # Accuracy
    ///
    /// Due to implementation details, the returned value can be off by 1 in
    /// either direction when other threads are manipulating any `Arc`s or
    /// `Weak`s pointing to the same allocation.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        if let Some(inner) = self.inner() {
            let weak = inner.weak.load(Acquire);
            let strong = inner.strong.load(Relaxed);
            if strong == 0 {
                0
            } else {
                // Since we observed that there was at least one strong pointer
                // after reading the weak count, we know that the implicit weak
                // reference (present whenever any strong references are alive)
                // was still around when we observed the weak count, and can
                // therefore safely subtract it.
                weak - 1
            }
        } else {
            0
        }
    }

    /// Returns `None` when the pointer is dangling and there is no allocated
    /// `ArcInner`.
    #[inline]
    fn inner(&self) -> Option<WeakInner<'_>> {
        let ptr = self.ptr.as_ptr();

        if is_dangling(ptr) {
            None
        } else {
            // We are careful to *not* create a reference covering the "data"
            // field, as the field may be mutated concurrently (for example, if
            // the last `Arc` is dropped, the data field will be dropped
            // in-place).
            Some(unsafe {
                WeakInner {
                    strong: &(*ptr).strong,
                    weak: &(*ptr).weak,
                }
            })
        }
    }
}

impl<T, A> Clone for Weak<T, A>
where
    T: ?Sized,
    A: Allocator + Clone,
{
    /// Makes a clone of the `Weak` pointer that points to the same allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::sync::{Arc, Weak};
    ///
    /// let weak_five = Arc::downgrade(&Arc::try_new(5)?);
    ///
    /// let _ = Weak::clone(&weak_five);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn clone(&self) -> Weak<T, A> {
        if let Some(inner) = self.inner() {
            // See comments in Arc::clone() for why this is relaxed. This can use a
            // fetch_add (ignoring the lock) because the weak count is only locked
            // where are *no other* weak pointers in existence. (So we can't be
            // running this code in that case).
            let old_size = inner.weak.fetch_add(1, Relaxed);

            // See comments in Arc::clone() for why we do this (for mem::forget).
            if old_size > MAX_REFCOUNT {
                abort();
            }
        }

        Weak {
            ptr: self.ptr,
            alloc: self.alloc.clone(),
        }
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
    /// use rune::alloc::sync::{Arc, Weak};
    ///
    /// struct Foo;
    ///
    /// impl Drop for Foo {
    ///     fn drop(&mut self) {
    ///         println!("dropped!");
    ///     }
    /// }
    ///
    /// let foo = Arc::try_new(Foo)?;
    /// let weak_foo = Arc::downgrade(&foo);
    /// let other_weak_foo = Weak::clone(&weak_foo);
    ///
    /// drop(weak_foo);   // Doesn't print anything
    /// drop(foo);        // Prints "dropped!"
    ///
    /// assert!(other_weak_foo.upgrade().is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn drop(&mut self) {
        // If we find out that we were the last weak pointer, then its time to
        // deallocate the data entirely. See the discussion in Arc::drop() about
        // the memory orderings
        //
        // It's not necessary to check for the locked state here, because the
        // weak count can only be locked if there was precisely one weak ref,
        // meaning that drop could only subsequently run ON that remaining weak
        // ref, which can only happen after the lock is released.
        let Some(inner) = self.inner() else {
            return;
        };

        if inner.weak.fetch_sub(1, Release) == 1 {
            acquire!(inner.weak);

            // Make sure we aren't trying to "deallocate" the shared static for empty slices
            // used by Default::default.
            debug_assert!(
                !ptr::addr_eq(self.ptr.as_ptr(), &STATIC_INNER_SLICE.inner),
                "Arc/Weaks backed by a static should never be deallocated. \
                Likely decrement_strong_count or from_raw were called too many times.",
            );

            unsafe {
                self.alloc
                    .deallocate(self.ptr.cast(), for_value_raw(self.ptr.as_ptr()))
            }
        }
    }
}

unsafe impl<T, A> Send for Weak<T, A>
where
    T: ?Sized + Sync + Send,
    A: Allocator + Send,
{
}
unsafe impl<T, A> Sync for Weak<T, A>
where
    T: ?Sized + Sync + Send,
    A: Allocator + Sync,
{
}

impl<T, A> fmt::Debug for Weak<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Weak)")
    }
}

// This is repr(C) to future-proof against possible field-reordering, which
// would interfere with otherwise safe [into|from]_raw() of transmutable
// inner types.
#[repr(C)]
struct ArcInner<T>
where
    T: ?Sized,
{
    strong: AtomicUsize,

    // the value usize::MAX acts as a sentinel for temporarily "locking" the
    // ability to upgrade weak pointers or downgrade strong ones; this is used
    // to avoid races in `make_mut` and `get_mut`.
    weak: AtomicUsize,

    data: T,
}

/// Calculate layout for `ArcInner<T>` using the inner value's layout
fn arcinner_layout_for_value_layout(layout: Layout) -> Layout {
    // Calculate layout using the given value layout.
    // Previously, layout was calculated on the expression
    // `&*(ptr as *const ArcInner<T>)`, but this created a misaligned
    // reference (see #54908).
    Layout::new::<ArcInner<()>>()
        .extend(layout)
        .unwrap()
        .0
        .pad_to_align()
}

unsafe impl<T> Send for ArcInner<T> where T: ?Sized + Sync + Send {}
unsafe impl<T> Sync for ArcInner<T> where T: ?Sized + Sync + Send {}

impl<T> Arc<T> {
    /// Constructs a new `Arc<T>`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails with an [`Error`][crate::error::Error].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::new(5);
    /// ```
    #[inline]
    #[deprecated = "Use `Arc::try_new` instead, which uses checked allocations."]
    pub fn new(data: T) -> Arc<T> {
        match Self::try_new_in(data, Global) {
            Ok(arc) => arc,
            Err(err) => panic!("{err}"),
        }
    }

    /// Constructs a new `Arc<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_new(data: T) -> Result<Arc<T>> {
        Self::try_new_in(data, Global)
    }
}

impl<T, A> Arc<T, A>
where
    A: Allocator,
{
    /// Constructs a new `Arc<T>` in the provided allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    /// use rune::alloc::alloc::Global;
    ///
    /// let five = Arc::try_new_in(5, Global)?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_new_in(data: T, alloc: A) -> Result<Arc<T, A>> {
        // Start the weak pointer count as 1 which is the weak pointer that's
        // held by all the strong pointers (kinda), see std/rc.rs for more info
        let x = Box::try_new_in(
            ArcInner {
                strong: AtomicUsize::new(1),
                weak: AtomicUsize::new(1),
                data,
            },
            alloc,
        )?;

        let (ptr, alloc) = Box::into_unique_with_allocator(x);
        Ok(unsafe { Self::from_inner_in(ptr.into(), alloc) })
    }
}

impl<T, A> Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Returns a reference to the underlying allocator.
    ///
    /// Note: this is an associated function, which means that you have to call
    /// it as `Arc::allocator(&a)` instead of `a.allocator()`. This is so that
    /// there is no conflict with a method on the inner type.
    #[inline]
    pub fn allocator(this: &Self) -> &A {
        &this.alloc
    }

    /// Consumes the `Arc`, returning the wrapped pointer and allocator.
    ///
    /// To avoid a memory leak the pointer must be converted back to an `Arc`
    /// using [`Arc::from_raw_in`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::prelude::*;
    /// use rune::sync::Arc;
    /// use rune::alloc::alloc::Global;
    ///
    /// let x = Arc::try_new_in("hello".try_to_owned()?, Global)?;
    /// let (ptr, alloc) = Arc::into_raw_with_allocator(x);
    /// assert_eq!(unsafe { &*ptr }, "hello");
    /// let x = unsafe { Arc::from_raw_in(ptr, alloc) };
    /// assert_eq!(&*x, "hello");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw_with_allocator(this: Self) -> (*const T, A) {
        let this = mem::ManuallyDrop::new(this);
        let ptr = Self::as_ptr(&this);
        // Safety: `this` is ManuallyDrop so the allocator will not be double-dropped
        let alloc = unsafe { ptr::read(&this.alloc) };
        (ptr, alloc)
    }

    /// Provides a raw pointer to the data.
    ///
    /// The counts are not affected in any way and the `Arc` is not consumed. The pointer is valid for
    /// as long as there are strong counts in the `Arc`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::prelude::*;
    /// use rune::sync::Arc;
    ///
    /// let x = Arc::try_new("hello".try_to_owned()?)?;
    /// let y = Arc::clone(&x);
    /// let x_ptr = Arc::as_ptr(&x);
    /// assert_eq!(x_ptr, Arc::as_ptr(&y));
    /// assert_eq!(unsafe { &*x_ptr }, "hello");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn as_ptr(this: &Self) -> *const T {
        let ptr: *mut ArcInner<T> = NonNull::as_ptr(this.ptr);

        // SAFETY: This cannot go through Deref::deref or RcInnerPtr::inner because
        // this is required to retain raw/mut provenance such that e.g. `get_mut` can
        // write through the pointer after the Rc is recovered through `from_raw`.
        unsafe { &raw mut (*ptr).data }
    }

    /// Constructs an `Arc<T, A>` from a raw pointer.
    ///
    /// The raw pointer has the following requirements:
    ///
    /// * If `U` is sized, it must have the same size and alignment as `T`. This
    ///   is trivially true if `U` is `T`.
    /// * If `U` is unsized, its data pointer must have the same size and
    ///   alignment as `T`. This is trivially true if `Arc<U>` was constructed
    ///   through `Arc<T>` and then converted to `Arc<U>` through an [unsized
    ///   coercion].
    ///
    /// Note that if `U` or `U`'s data pointer is not `T` but has the same size
    /// and alignment, this is basically like transmuting references of
    /// different types. See [`mem::transmute`] for more information on what
    /// restrictions apply in this case.
    ///
    /// The raw pointer must point to a block of memory allocated by `alloc`
    ///
    /// The user of `from_raw` has to make sure a specific value of `T` is only
    /// dropped once.
    ///
    /// This function is unsafe because improper use may lead to memory
    /// unsafety, even if the returned `Arc<T>` is never accessed.
    ///
    /// [unsized coercion]:
    ///     https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions
    ///
    /// # Safety
    ///
    /// The pointer must point to an instance which has previously ben returned
    /// by [`Arc<T>::into_raw_with_allocator`]. The allocator that was used must
    /// also be compatible.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::prelude::*;
    /// use rune::sync::Arc;
    /// use rune::alloc::alloc::Global;
    ///
    /// let x = Arc::try_new_in("hello".try_to_owned()?, Global)?;
    /// let (x_ptr, alloc) = Arc::into_raw_with_allocator(x);
    ///
    /// unsafe {
    ///     // Convert back to an `Arc` to prevent leak.
    ///     let x = Arc::from_raw_in(x_ptr, alloc);
    ///     assert_eq!(&*x, "hello");
    ///
    ///     // Further calls to `Arc::from_raw(x_ptr)` would be memory-unsafe.
    /// }
    ///
    /// // The memory was freed when `x` went out of scope above, so `x_ptr` is now dangling!
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Convert a slice back into its original array:
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let original: &[u8] = &[1, 2, 3];
    /// let x: Arc<[u8]> = Arc::try_from(original)?;
    /// let (x_ptr, alloc) = Arc::into_raw_with_allocator(x);
    ///
    /// unsafe {
    ///     let x: Arc<[u8; 3], _> = Arc::from_raw_in(x_ptr.cast::<[u8; 3]>(), alloc);
    ///     assert_eq!(&*x, &[1, 2, 3]);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub unsafe fn from_raw_in(ptr: *const T, alloc: A) -> Self {
        unsafe {
            let offset = data_offset(ptr);
            // Reverse the offset to find the original ArcInner.
            let arc_ptr = ptr.byte_sub(offset) as *mut ArcInner<T>;
            Self::from_ptr_in(arc_ptr, alloc)
        }
    }

    /// Returns `true` if the two `Arc`s point to the same allocation in a vein
    /// similar to [`ptr::eq`]. This function ignores the metadata of  `dyn
    /// Trait` pointers.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    /// let same_five = Arc::clone(&five);
    /// let other_five = Arc::try_new(5)?;
    ///
    /// assert!(Arc::ptr_eq(&five, &same_five));
    /// assert!(!Arc::ptr_eq(&five, &other_five));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`ptr::eq`]: core::ptr::eq "ptr::eq"
    #[inline]
    #[must_use]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        ptr::addr_eq(this.ptr.as_ptr(), other.ptr.as_ptr())
    }

    /// Returns a mutable reference into the given `Arc`, if there are
    /// no other `Arc` or [`Weak`] pointers to the same allocation.
    ///
    /// Returns [`None`] otherwise, because it is not safe to
    /// mutate a shared value.
    ///
    /// [clone]: Clone::clone
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let mut x = Arc::try_new(3)?;
    /// *Arc::get_mut(&mut x).unwrap() = 4;
    /// assert_eq!(*x, 4);
    ///
    /// let _y = Arc::clone(&x);
    /// assert!(Arc::get_mut(&mut x).is_none());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn get_mut(this: &mut Self) -> Option<&mut T> {
        if Self::is_unique(this) {
            // This unsafety is ok because we're guaranteed that the pointer
            // returned is the *only* pointer that will ever be returned to T. Our
            // reference count is guaranteed to be 1 at this point, and we required
            // the Arc itself to be `mut`, so we're returning the only possible
            // reference to the inner data.
            unsafe { Some(Arc::get_mut_unchecked(this)) }
        } else {
            None
        }
    }

    /// Returns a mutable reference into the given `Arc`, without any check.
    ///
    /// See also [`get_mut`], which is safe and does appropriate checks.
    ///
    /// [`get_mut`]: Arc::get_mut
    ///
    /// # Safety
    ///
    /// If any other `Arc` or [`Weak`] pointers to the same allocation exist,
    /// then they must not be dereferenced or have active borrows for the
    /// duration of the returned borrow, and their inner type must be exactly
    /// the same as the inner type of this Rc (including lifetimes). This is
    /// trivially the case if no such pointers exist, for example immediately
    /// after `Arc::new`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    /// use rune::alloc::String;
    ///
    /// let mut x = Arc::try_new(String::new())?;
    ///
    /// unsafe {
    ///     Arc::get_mut_unchecked(&mut x).try_push_str("foo")?
    /// }
    ///
    /// assert_eq!(*x, "foo");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Other `Arc` pointers to the same allocation must be to the same type.
    ///
    /// ```no_run
    /// use rune::sync::Arc;
    ///
    /// let x: Arc<str> = Arc::try_from("Hello, world!")?;
    /// let mut y: Arc<[u8]> = x.clone().try_into()?;
    ///
    /// unsafe {
    ///     // this is Undefined Behavior, because x's inner type is str, not [u8]
    ///     Arc::get_mut_unchecked(&mut y).fill(0xff); // 0xff is invalid in UTF-8
    /// }
    ///
    /// println!("{}", &*x); // Invalid UTF-8 in a str
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Other `Arc` pointers to the same allocation must be to the exact same
    /// type, including lifetimes.
    ///
    /// ```no_run
    /// use rune::sync::Arc;
    ///
    /// let x: Arc<&str> = Arc::try_new("Hello, world!")?;
    ///
    /// {
    ///     let s = String::from("Oh, no!");
    ///     let mut y: Arc<&str> = x.clone();
    ///     unsafe {
    ///         // this is Undefined Behavior, because x's inner type
    ///         // is &'long str, not &'short str
    ///         *Arc::get_mut_unchecked(&mut y) = &s;
    ///     }
    /// }
    ///
    /// println!("{}", &*x); // Use-after-free
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub unsafe fn get_mut_unchecked(this: &mut Self) -> &mut T {
        // We are careful to *not* create a reference covering the "count" fields, as
        // this would alias with concurrent access to the reference counts (e.g. by `Weak`).
        unsafe { &mut (*this.ptr.as_ptr()).data }
    }

    /// Determine whether this is the unique reference to the underlying data.
    ///
    /// Returns `true` if there are no other `Arc` or [`Weak`] pointers to the same allocation;
    /// returns `false` otherwise.
    ///
    /// If this function returns `true`, then is guaranteed to be safe to call [`get_mut_unchecked`]
    /// on this `Arc`, so long as no clones occur in between.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let x = Arc::try_new(3)?;
    /// assert!(Arc::is_unique(&x));
    ///
    /// let y = Arc::clone(&x);
    /// assert!(!Arc::is_unique(&x));
    /// drop(y);
    ///
    /// // Weak references also count, because they could be upgraded at any time.
    /// let z = Arc::downgrade(&x);
    /// assert!(!Arc::is_unique(&x));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// # Pointer invalidation
    ///
    /// This function will always return the same value as `Arc::get_mut(arc).is_some()`. However,
    /// unlike that operation it does not produce any mutable references to the underlying data,
    /// meaning no pointers to the data inside the `Arc` are invalidated by the call. Thus, the
    /// following code is valid, even though it would be UB if it used `Arc::get_mut`:
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let arc = Arc::try_new(5)?;
    /// let pointer: *const i32 = &*arc;
    /// assert!(Arc::is_unique(&arc));
    /// assert_eq!(unsafe { *pointer }, 5);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// # Atomic orderings
    ///
    /// Concurrent drops to other `Arc` pointers to the same allocation will synchronize with this
    /// call - that is, this call performs an `Acquire` operation on the underlying strong and weak
    /// ref counts. This ensures that calling `get_mut_unchecked` is safe.
    ///
    /// Note that this operation requires locking the weak ref count, so concurrent calls to
    /// `downgrade` may spin-loop for a short period of time.
    ///
    /// [`get_mut_unchecked`]: Self::get_mut_unchecked
    #[inline]
    pub fn is_unique(this: &Self) -> bool {
        // lock the weak pointer count if we appear to be the sole weak pointer
        // holder.
        //
        // The acquire label here ensures a happens-before relationship with any
        // writes to `strong` (in particular in `Weak::upgrade`) prior to decrements
        // of the `weak` count (via `Weak::drop`, which uses release). If the upgraded
        // weak ref was never dropped, the CAS here will fail so we do not care to synchronize.
        if this
            .inner()
            .weak
            .compare_exchange(1, usize::MAX, Acquire, Relaxed)
            .is_ok()
        {
            // This needs to be an `Acquire` to synchronize with the decrement of the `strong`
            // counter in `drop` -- the only access that happens when any but the last reference
            // is being dropped.
            let unique = this.inner().strong.load(Acquire) == 1;

            // The release write here synchronizes with a read in `downgrade`,
            // effectively preventing the above read of `strong` from happening
            // after the write.
            this.inner().weak.store(1, Release); // release the lock
            unique
        } else {
            false
        }
    }

    /// Creates a new [`Weak`] pointer to this allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// let weak_five = Arc::downgrade(&five);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "this returns a new `Weak` pointer, \
                  without modifying the original `Arc`"]
    pub fn downgrade(this: &Self) -> Weak<T, A>
    where
        A: Clone,
    {
        // This Relaxed is OK because we're checking the value in the CAS
        // below.
        let mut cur = this.inner().weak.load(Relaxed);

        loop {
            // check if the weak counter is currently "locked"; if so, spin.
            if cur == usize::MAX {
                hint::spin_loop();
                cur = this.inner().weak.load(Relaxed);
                continue;
            }

            // We can't allow the refcount to increase much past `MAX_REFCOUNT`.
            assert!(cur <= MAX_REFCOUNT, "{}", INTERNAL_OVERFLOW_ERROR);

            // NOTE: this code currently ignores the possibility of overflow
            // into usize::MAX; in general both Rc and Arc need to be adjusted
            // to deal with overflow.

            // Unlike with Clone(), we need this to be an Acquire read to
            // synchronize with the write coming from `is_unique`, so that the
            // events prior to that write happen before this read.
            match this
                .inner()
                .weak
                .compare_exchange_weak(cur, cur + 1, Acquire, Relaxed)
            {
                Ok(_) => {
                    // Make sure we do not create a dangling Weak
                    debug_assert!(!is_dangling(this.ptr.as_ptr()));
                    return Weak {
                        ptr: this.ptr,
                        alloc: this.alloc.clone(),
                    };
                }
                Err(old) => cur = old,
            }
        }
    }

    /// Allocates an `ArcInner<T>` with sufficient space for
    /// a possibly-unsized inner value where the value has the layout provided.
    ///
    /// The function `mem_to_arcinner` is called with the data pointer
    /// and must return back a (potentially fat)-pointer for the `ArcInner<T>`.
    unsafe fn try_allocate_for_layout(
        value_layout: Layout,
        allocate: impl FnOnce(Layout) -> Result<NonNull<[u8]>, AllocError>,
        mem_to_arcinner: impl FnOnce(*mut u8) -> *mut ArcInner<T>,
    ) -> Result<*mut ArcInner<T>, AllocError> {
        let layout = arcinner_layout_for_value_layout(value_layout);
        let ptr = allocate(layout)?;
        Ok(unsafe { Self::initialize_arcinner(ptr, layout, mem_to_arcinner) })
    }

    unsafe fn initialize_arcinner(
        ptr: NonNull<[u8]>,
        layout: Layout,
        mem_to_arcinner: impl FnOnce(*mut u8) -> *mut ArcInner<T>,
    ) -> *mut ArcInner<T> {
        let inner = mem_to_arcinner(ptr.cast().as_ptr());
        // TODO: Switch to `Layout::for_value_raw` once stable.
        debug_assert_eq!(unsafe { Layout::for_value(&*inner) }, layout);

        unsafe {
            (&raw mut (*inner).strong).write(AtomicUsize::new(1));
            (&raw mut (*inner).weak).write(AtomicUsize::new(1));
        }

        inner
    }

    #[inline]
    unsafe fn from_inner_in(ptr: NonNull<ArcInner<T>>, alloc: A) -> Self {
        Self {
            ptr,
            phantom: PhantomData,
            alloc,
        }
    }

    #[inline]
    unsafe fn from_ptr_in(ptr: *mut ArcInner<T>, alloc: A) -> Self {
        unsafe { Self::from_inner_in(NonNull::new_unchecked(ptr), alloc) }
    }

    #[inline]
    fn inner(&self) -> &ArcInner<T> {
        // This unsafety is ok because while this arc is alive we're guaranteed
        // that the inner pointer is valid. Furthermore, we know that the
        // `ArcInner` structure itself is `Sync` because the inner data is
        // `Sync` as well, so we're ok loaning out an immutable pointer to these
        // contents.
        unsafe { self.ptr.as_ref() }
    }

    // Non-inlined part of `drop`.
    #[inline(never)]
    unsafe fn drop_slow(&mut self) {
        // Drop the weak ref collectively held by all strong references when this
        // variable goes out of scope. This ensures that the memory is deallocated
        // even if the destructor of `T` panics.
        // Take a reference to `self.alloc` instead of cloning because 1. it'll last long
        // enough, and 2. you should be able to drop `Arc`s with unclonable allocators
        let _weak = Weak {
            ptr: self.ptr,
            alloc: &self.alloc,
        };

        // Destroy the data at this time, even though we must not free the box
        // allocation itself (there might still be weak pointers lying around).
        // We cannot use `get_mut_unchecked` here, because `self.alloc` is borrowed.
        unsafe { ptr::drop_in_place(&mut (*self.ptr.as_ptr()).data) };
    }
}

impl<T, A> Clone for Arc<T, A>
where
    T: ?Sized,
    A: Allocator + Clone,
{
    /// Makes a clone of the `Arc` pointer.
    ///
    /// This creates another pointer to the same allocation, increasing the
    /// strong reference count.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// let _ = Arc::clone(&five);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn clone(&self) -> Arc<T, A> {
        // Using a relaxed ordering is alright here, as knowledge of the
        // original reference prevents other threads from erroneously deleting
        // the object.
        //
        // As explained in the [Boost documentation][1], Increasing the
        // reference counter can always be done with memory_order_relaxed: New
        // references to an object can only be formed from an existing
        // reference, and passing an existing reference from one thread to
        // another must already provide any required synchronization.
        //
        // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
        let old_size = self.inner().strong.fetch_add(1, Relaxed);

        // However we need to guard against massive refcounts in case someone is `mem::forget`ing
        // Arcs. If we don't do this the count can overflow and users will use-after free. This
        // branch will never be taken in any realistic program. We abort because such a program is
        // incredibly degenerate, and we don't care to support it.
        //
        // This check is not 100% water-proof: we error when the refcount grows beyond `isize::MAX`.
        // But we do that check *after* having done the increment, so there is a chance here that
        // the worst already happened and we actually do overflow the `usize` counter. However, that
        // requires the counter to grow from `isize::MAX` to `usize::MAX` between the increment
        // above and the `abort` below, which seems exceedingly unlikely.
        //
        // This is a global invariant, and also applies when using a compare-exchange loop to increment
        // counters in other methods.
        // Otherwise, the counter could be brought to an almost-overflow using a compare-exchange loop,
        // and then overflow using a few `fetch_add`s.
        if old_size > MAX_REFCOUNT {
            abort();
        }

        unsafe { Self::from_inner_in(self.ptr, self.alloc.clone()) }
    }
}

impl<T, A> TryClone for Arc<T, A>
where
    T: ?Sized,
    A: Allocator + Clone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

impl<T, A> Deref for Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.inner().data
    }
}

impl<T, A> Borrow<T> for Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    #[inline]
    fn borrow(&self) -> &T {
        self
    }
}

impl<T, A> AsRef<T> for Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T, A> Drop for Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    /// Drops the `Arc`.
    ///
    /// This will decrement the strong reference count. If the strong reference
    /// count reaches zero then the only other references (if any) are
    /// [`Weak`], so we `drop` the inner value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// struct Foo;
    ///
    /// impl Drop for Foo {
    ///     fn drop(&mut self) {
    ///         println!("dropped!");
    ///     }
    /// }
    ///
    /// let foo  = Arc::try_new(Foo)?;
    /// let foo2 = Arc::clone(&foo);
    ///
    /// drop(foo);    // Doesn't print anything
    /// drop(foo2);   // Prints "dropped!"
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn drop(&mut self) {
        // Because `fetch_sub` is already atomic, we do not need to synchronize
        // with other threads unless we are going to delete the object. This
        // same logic applies to the below `fetch_sub` to the `weak` count.
        if self.inner().strong.fetch_sub(1, Release) != 1 {
            return;
        }

        // This fence is needed to prevent reordering of use of the data and
        // deletion of the data. Because it is marked `Release`, the decreasing
        // of the reference count synchronizes with this `Acquire` fence. This
        // means that use of the data happens before decreasing the reference
        // count, which happens before this fence, which happens before the
        // deletion of the data.
        //
        // As explained in the [Boost documentation][1],
        //
        // > It is important to enforce any possible access to the object in one
        // > thread (through an existing reference) to *happen before* deleting
        // > the object in a different thread. This is achieved by a "release"
        // > operation after dropping a reference (any access to the object
        // > through this reference must obviously happened before), and an
        // > "acquire" operation before deleting the object.
        //
        // In particular, while the contents of an Arc are usually immutable, it's
        // possible to have interior writes to something like a Mutex<T>. Since a
        // Mutex is not acquired when it is deleted, we can't rely on its
        // synchronization logic to make writes in thread A visible to a destructor
        // running in thread B.
        //
        // Also note that the Acquire fence here could probably be replaced with an
        // Acquire load, which could improve performance in highly-contended
        // situations. See [2].
        //
        // [1]: (www.boost.org/doc/libs/1_55_0/doc/html/atomic/usage_examples.html)
        // [2]: (https://github.com/rust-lang/rust/pull/41714)
        acquire!(self.inner().strong);

        // Make sure we aren't trying to "drop" the shared static for empty
        // slices used by Default::default.
        debug_assert!(
            !ptr::addr_eq(self.ptr.as_ptr(), &STATIC_INNER_SLICE.inner),
            "Arcs backed by a static should never reach a strong count of 0. \
            Likely decrement_strong_count or from_raw were called too many times.",
        );

        unsafe {
            self.drop_slow();
        }
    }
}

impl<T, A> PartialEq for Arc<T, A>
where
    T: ?Sized + PartialEq,
    A: Allocator,
{
    /// Equality for two `Arc`s.
    ///
    /// Two `Arc`s are equal if their inner values are equal, even if they are
    /// stored in different allocation.
    ///
    /// If `T` also implements `Eq` (implying reflexivity of equality),
    /// two `Arc`s that point to the same allocation are always equal.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five == Arc::try_new(5)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn eq(&self, other: &Arc<T, A>) -> bool {
        Arc::ptr_eq(self, other) || **self == **other
    }

    /// Inequality for two `Arc`s.
    ///
    /// Two `Arc`s are not equal if their inner values are not equal.
    ///
    /// If `T` also implements `Eq` (implying reflexivity of equality),
    /// two `Arc`s that point to the same value are always equal.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five != Arc::try_new(6)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[allow(clippy::partialeq_ne_impl)]
    #[inline]
    fn ne(&self, other: &Arc<T, A>) -> bool {
        !Arc::ptr_eq(self, other) && **self != **other
    }
}

impl<T: ?Sized + PartialOrd, A: Allocator> PartialOrd for Arc<T, A> {
    /// Partial comparison for two `Arc`s.
    ///
    /// The two are compared by calling `partial_cmp()` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::cmp::Ordering;
    ///
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert_eq!(Some(Ordering::Less), five.partial_cmp(&Arc::try_new(6)?));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn partial_cmp(&self, other: &Arc<T, A>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }

    /// Less-than comparison for two `Arc`s.
    ///
    /// The two are compared by calling `<` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five < Arc::try_new(6)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn lt(&self, other: &Arc<T, A>) -> bool {
        *(*self) < *(*other)
    }

    /// 'Less than or equal to' comparison for two `Arc`s.
    ///
    /// The two are compared by calling `<=` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five <= Arc::try_new(5)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn le(&self, other: &Arc<T, A>) -> bool {
        *(*self) <= *(*other)
    }

    /// Greater-than comparison for two `Arc`s.
    ///
    /// The two are compared by calling `>` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five > Arc::try_new(4)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn gt(&self, other: &Arc<T, A>) -> bool {
        *(*self) > *(*other)
    }

    /// 'Greater than or equal to' comparison for two `Arc`s.
    ///
    /// The two are compared by calling `>=` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert!(five >= Arc::try_new(5)?);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn ge(&self, other: &Arc<T, A>) -> bool {
        *(*self) >= *(*other)
    }
}

impl<T: ?Sized + Ord, A: Allocator> Ord for Arc<T, A> {
    /// Comparison for two `Arc`s.
    ///
    /// The two are compared by calling `cmp()` on their inner values.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::sync::Arc;
    /// use std::cmp::Ordering;
    ///
    /// let five = Arc::try_new(5)?;
    ///
    /// assert_eq!(Ordering::Less, five.cmp(&Arc::try_new(6)?));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn cmp(&self, other: &Arc<T, A>) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<T, A> Eq for Arc<T, A>
where
    T: ?Sized + Eq,
    A: Allocator,
{
}

impl<T, A> fmt::Display for Arc<T, A>
where
    T: ?Sized + fmt::Display,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T, A> fmt::Debug for Arc<T, A>
where
    T: ?Sized + fmt::Debug,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, A> fmt::Pointer for Arc<T, A>
where
    T: ?Sized,
    A: Allocator,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(&raw const **self), f)
    }
}

impl<T, A> Hash for Arc<T, A>
where
    T: ?Sized + Hash,
    A: Allocator,
{
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        (**self).hash(state)
    }
}

impl<A> TryFrom<&[u8]> for Arc<[u8], A>
where
    A: Default + Allocator,
{
    type Error = AllocError;

    /// Allocates a reference-counted slice and fills it by cloning `v`'s items.
    ///
    /// # Example
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let original: &[u8] = &[1, 2, 3];
    /// let shared: Arc<[u8]> = Arc::try_from(original)?;
    /// assert_eq!(&[1, 2, 3], &shared[..]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        // SAFETY: `T` is Copy.
        Arc::copy_from_slice_in(v, A::default())
    }
}

impl<T, A: Allocator> TryFrom<Vec<T, A>> for Arc<[T], A> {
    type Error = AllocError;

    /// Allocates a reference-counted slice and moves `v`'s items into it.
    ///
    /// # Example
    ///
    /// ```
    /// use rune::sync::Arc;
    /// use rune::alloc::{try_vec, Vec};
    ///
    /// let unique: Vec<i32> = try_vec![1, 2, 3];
    /// let shared: Arc<[i32]> = Arc::try_from(unique)?;
    /// assert_eq!(&[1, 2, 3], &shared[..]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(v: Vec<T, A>) -> Result<Arc<[T], A>, Self::Error> {
        unsafe {
            let (vec_ptr, len, cap, alloc) = v.into_raw_parts_with_alloc();

            let rc_ptr = Self::try_allocate_for_slice_in(len, &alloc)?;
            ptr::copy_nonoverlapping(vec_ptr, (&raw mut (*rc_ptr).data) as *mut T, len);

            // Create a `Vec<T, &A>` with length 0, to deallocate the buffer
            // without dropping its contents or the allocator
            let _ = Vec::from_raw_parts_in(vec_ptr, 0, cap, &alloc);
            Ok(Self::from_ptr_in(rc_ptr, alloc))
        }
    }
}

impl<A> TryFrom<&str> for Arc<str, A>
where
    A: Default + Allocator,
{
    type Error = AllocError;

    /// Allocates a reference-counted `str` and copies `v` into it.
    ///
    /// # Example
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let shared: Arc<str> = Arc::try_from("eggplant")?;
    /// assert_eq!("eggplant", &shared[..]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let arc = Arc::try_from(v.as_bytes())?;
        let (ptr, alloc) = Arc::into_raw_with_allocator(arc);
        Ok(unsafe { Arc::from_raw_in(ptr as *const str, alloc) })
    }
}

impl<A> From<Arc<str, A>> for Arc<[u8], A>
where
    A: Allocator,
{
    /// Converts an atomically reference-counted string slice into a byte slice.
    ///
    /// # Example
    ///
    /// ```
    /// use rune::sync::Arc;
    ///
    /// let string: Arc<str> = Arc::try_from("eggplant")?;
    /// let bytes: Arc<[u8]> = Arc::from(string);
    /// assert_eq!("eggplant".as_bytes(), bytes.as_ref());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn from(arc: Arc<str, A>) -> Self {
        // SAFETY: `str` has the same layout as `[u8]`.
        let (ptr, alloc) = Arc::into_raw_with_allocator(arc);
        unsafe { Arc::from_raw_in(ptr as *const [u8], alloc) }
    }
}

/// Gets the offset within an `ArcInner` for the payload behind a pointer.
///
/// # Safety
///
/// The pointer must point to (and have valid metadata for) a previously
/// valid instance of T, but the T is allowed to be dropped.
unsafe fn data_offset<T>(ptr: *const T) -> usize
where
    T: ?Sized,
{
    // Align the unsized value to the end of the ArcInner. Because RcInner is
    // repr(C), it will always be the last field in memory. SAFETY: since the
    // only unsized types possible are slices, trait objects, and extern types,
    // the input safety requirement is currently enough to satisfy the
    // requirements of align_of_val_raw; this is an implementation detail of the
    // language that must not be relied upon outside of std.
    // TODO: Switch to `align_of_val_raw` once stable.
    unsafe { data_offset_align(align_of_val(&*ptr)) }
}

#[inline]
fn data_offset_align(align: usize) -> usize {
    let layout = Layout::new::<ArcInner<()>>();
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

/// Struct to hold the static `ArcInner` used for empty `Arc<str/CStr/[T]>` as
/// returned by `Default::default`.
///
/// Layout notes:
/// * `repr(align(16))` so we can use it for `[T]` with `align_of::<T>() <= 16`.
/// * `repr(C)` so `inner` is at offset 0 (and thus guaranteed to actually be
///   aligned to 16).
/// * `[u8; 1]` (to be initialized with 0) so it can be used for `Arc<CStr>`.
#[repr(C, align(16))]
struct SliceArcInnerForStatic {
    inner: ArcInner<[u8; 1]>,
}

static STATIC_INNER_SLICE: SliceArcInnerForStatic = SliceArcInnerForStatic {
    inner: ArcInner {
        strong: AtomicUsize::new(1),
        weak: AtomicUsize::new(1),
        data: [0],
    },
};

//! A contiguous growable array type with heap-allocated contents, written
//! `Vec<T>`.
//!
//! Vectors have *O*(1) indexing, amortized *O*(1) push (to the end) and
//! *O*(1) pop (from the end).
//!
//! Vectors ensure they never allocate more than `isize::MAX` bytes.
//!
//! # Examples
//!
//! You can explicitly create a [`Vec`] with [`Vec::new`]:
//!
//! ```
//! use rune::alloc::Vec;
//!
//! let v: Vec<i32> = Vec::new();
//! ```
//!
//! ...or by using the [`try_vec!`][crate::try_vec!] macro:
//!
//! ```
//! use rune::alloc::{try_vec, Vec};
//!
//! let v: Vec<i32> = try_vec![];
//! let v = try_vec![1, 2, 3, 4, 5];
//! let v = try_vec![0; 10]; // ten zeroes
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! You can [`try_push`] values onto the end of a vector (which will grow the vector
//! as needed):
//!
//! ```
//! use rune::alloc::try_vec;
//! let mut v = try_vec![1, 2];
//!
//! v.try_push(3)?;
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! Popping values works in much the same way:
//!
//! ```
//! use rune::alloc::try_vec;
//!
//! let mut v = try_vec![1, 2];
//!
//! let two = v.pop();
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! Vectors also support indexing (through the [`Index`] and [`IndexMut`] traits):
//!
//! ```
//! use rune::alloc::try_vec;
//!
//! let mut v = try_vec![1, 2, 3];
//! let three = v[2];
//! v[1] = v[1] + 5;
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! [`try_push`]: Vec::try_push

pub use self::drain::Drain;

mod drain;
pub use self::into_iter::IntoIter;

mod into_iter;

mod partial_eq;

use self::spec_from_elem::SpecFromElem;
mod spec_from_elem;

use self::spec_extend::SpecExtend;
mod spec_extend;

use self::set_len_on_drop::SetLenOnDrop;
mod set_len_on_drop;

mod splice;

#[cfg(rune_nightly)]
use self::is_zero::IsZero;
#[cfg(rune_nightly)]
mod is_zero;

#[cfg(feature = "alloc")]
use core::alloc::Layout;
use core::borrow::Borrow;
use core::cmp;
use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::iter;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ops::{self, Index, IndexMut, Range, RangeBounds};
use core::slice::{self, SliceIndex};

use crate::alloc::{Allocator, Global, SizedTypeProperties};
use crate::clone::TryClone;
use crate::error::Error;
use crate::iter::{TryExtend, TryFromIteratorIn};
use crate::ptr::{self, NonNull};
use crate::raw_vec::RawVec;
use crate::slice::range as slice_range;
use crate::slice::{RawIter, RawIterMut};
#[cfg(test)]
use crate::testing::*;
use crate::Box;

/// Construct a vector from an element that can be cloned.
#[doc(hidden)]
pub fn try_from_elem<T: TryClone>(elem: T, n: usize) -> Result<Vec<T>, Error> {
    <T as SpecFromElem>::from_elem(elem, n, Global)
}

/// A contiguous growable array type, written as `Vec<T>`, short for 'vector'.
///
/// # Examples
///
/// ```
/// use rune::alloc::Vec;
/// use rune::alloc::prelude::*;
///
/// let mut vec = Vec::new();
/// vec.try_push(1)?;
/// vec.try_push(2)?;
///
/// assert_eq!(vec.len(), 2);
/// assert_eq!(vec[0], 1);
///
/// assert_eq!(vec.pop(), Some(2));
/// assert_eq!(vec.len(), 1);
///
/// vec[0] = 7;
/// assert_eq!(vec[0], 7);
///
/// vec.try_extend([1, 2, 3])?;
///
/// for x in &vec {
///     println!("{x}");
/// }
///
/// assert_eq!(vec, [7, 1, 2, 3]);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// The [`try_vec!`][crate::try_vec!] macro is provided for convenient
/// initialization:
///
/// ```
/// use rune::alloc::{try_vec, Vec};
///
/// let mut vec1 = try_vec![1, 2, 3];
/// vec1.try_push(4)?;
/// let vec2 = Vec::try_from([1, 2, 3, 4])?;
/// assert_eq!(vec1, vec2);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// It can also initialize each element of a `Vec<T>` with a given value.
/// This may be more efficient than performing allocation and initialization
/// in separate steps, especially when initializing a vector of zeros:
///
/// ```
/// use rune::alloc::{try_vec, Vec};
///
/// let vec = try_vec![0; 5];
/// assert_eq!(vec, [0, 0, 0, 0, 0]);
///
/// // The following is equivalent, but potentially slower:
/// let mut vec = Vec::try_with_capacity(5)?;
/// vec.try_resize(5, 0)?;
/// assert_eq!(vec, [0, 0, 0, 0, 0]);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// For more information, see
/// [Capacity and Reallocation](#capacity-and-reallocation).
///
/// Use a `Vec<T>` as an efficient stack:
///
/// ```
/// use rune::alloc::Vec;
///
/// let mut stack = Vec::new();
///
/// stack.try_push(1)?;
/// stack.try_push(2)?;
/// stack.try_push(3)?;
///
/// while let Some(top) = stack.pop() {
///     // Prints 3, 2, 1
///     println!("{top}");
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// # Indexing
///
/// The `Vec` type allows to access values by index, because it implements the
/// [`Index`] trait. An example will be more explicit:
///
/// ```
/// use rune::alloc::try_vec;
///
/// let v = try_vec![0, 2, 4, 6];
/// println!("{}", v[1]); // it will display '2'
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// However be careful: if you try to access an index which isn't in the `Vec`,
/// your software will panic! You cannot do this:
///
/// ```should_panic
/// use rune::alloc::try_vec;
///
/// let v = try_vec![0, 2, 4, 6];
/// println!("{}", v[6]); // it will panic!
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Use [`get`] and [`get_mut`] if you want to check whether the index is in
/// the `Vec`.
///
/// # Slicing
///
/// A `Vec` can be mutable. On the other hand, slices are read-only objects.
/// To get a [slice][prim@slice], use [`&`]. Example:
///
/// ```
/// use rune::alloc::try_vec;
///
/// fn read_slice(slice: &[usize]) {
///     // ...
/// }
///
/// let v = try_vec![0, 1];
/// read_slice(&v);
///
/// // ... and that's all!
/// // you can also do it like this:
/// let u: &[usize] = &v;
/// // or like this:
/// let u: &[_] = &v;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// In Rust, it's more common to pass slices as arguments rather than vectors
/// when you just want to provide read access. The same goes for [`String`] and
/// [`&str`].
///
/// # Capacity and reallocation
///
/// The capacity of a vector is the amount of space allocated for any future
/// elements that will be added onto the vector. This is not to be confused with
/// the *length* of a vector, which specifies the number of actual elements
/// within the vector. If a vector's length exceeds its capacity, its capacity
/// will automatically be increased, but its elements will have to be
/// reallocated.
///
/// For example, a vector with capacity 10 and length 0 would be an empty vector
/// with space for 10 more elements. Pushing 10 or fewer elements onto the
/// vector will not change its capacity or cause reallocation to occur. However,
/// if the vector's length is increased to 11, it will have to reallocate, which
/// can be slow. For this reason, it is recommended to use
/// [`Vec::try_with_capacity`] whenever possible to specify how big the vector
/// is expected to get.
///
/// # Guarantees
///
/// Due to its incredibly fundamental nature, `Vec` makes a lot of guarantees
/// about its design. This ensures that it's as low-overhead as possible in
/// the general case, and can be correctly manipulated in primitive ways
/// by unsafe code. Note that these guarantees refer to an unqualified `Vec<T>`.
/// If additional type parameters are added (e.g., to support custom allocators),
/// overriding their defaults may change the behavior.
///
/// Most fundamentally, `Vec` is and always will be a (pointer, capacity, length)
/// triplet. No more, no less. The order of these fields is completely
/// unspecified, and you should use the appropriate methods to modify these.
/// The pointer will never be null, so this type is null-pointer-optimized.
///
/// However, the pointer might not actually point to allocated memory. In
/// particular, if you construct a `Vec` with capacity 0 via [`Vec::new`],
/// [`try_vec![]`], [`Vec::try_with_capacity(0)`], or by calling
/// [`try_shrink_to_fit`] on an empty Vec, it will not allocate memory.
/// Similarly, if you store zero-sized types inside a `Vec`, it will not
/// allocate space for them. *Note that in this case the `Vec` might not report
/// a [`capacity`] of 0*. `Vec` will allocate if and only if
/// <code>[mem::size_of::\<T>]\() * [capacity]\() > 0</code>. In general,
/// `Vec`'s allocation details are very subtle --- if you intend to allocate
/// memory using a `Vec` and use it for something else (either to pass to unsafe
/// code, or to build your own memory-backed collection), be sure to deallocate
/// this memory by using `from_raw_parts` to recover the `Vec` and then dropping
/// it.
///
/// [`try_vec![]`]: try_vec!
/// [`Vec::try_with_capacity(0)`]: Vec::try_with_capacity
///
/// If a `Vec` *has* allocated memory, then the memory it points to is on the heap
/// (as defined by the allocator Rust is configured to use by default), and its
/// pointer points to [`len`] initialized, contiguous elements in order (what
/// you would see if you coerced it to a slice), followed by <code>[capacity] - [len]</code>
/// logically uninitialized, contiguous elements.
///
/// A vector containing the elements `'a'` and `'b'` with capacity 4 can be
/// visualized as below. The top part is the `Vec` struct, it contains a
/// pointer to the head of the allocation in the heap, length and capacity.
/// The bottom part is the allocation on the heap, a contiguous memory block.
///
/// ```text
///             ptr      len  capacity
///        +--------+--------+--------+
///        | 0x0123 |      2 |      4 |
///        +--------+--------+--------+
///             |
///             v
/// Heap   +--------+--------+--------+--------+
///        |    'a' |    'b' | uninit | uninit |
///        +--------+--------+--------+--------+
/// ```
///
/// - **uninit** represents memory that is not initialized, see [`MaybeUninit`].
/// - Note: the ABI is not stable and `Vec` makes no guarantees about its memory
///   layout (including the order of fields).
///
/// `Vec` will never perform a "small optimization" where elements are actually
/// stored on the stack for two reasons:
///
/// * It would make it more difficult for unsafe code to correctly manipulate
///   a `Vec`. The contents of a `Vec` wouldn't have a stable address if it were
///   only moved, and it would be more difficult to determine if a `Vec` had
///   actually allocated memory.
///
/// * It would penalize the general case, incurring an additional branch
///   on every access.
///
/// `Vec` will never automatically shrink itself, even if completely empty. This
/// ensures no unnecessary allocations or deallocations occur. Emptying a `Vec`
/// and then filling it back up to the same [`len`] should incur no calls to the
/// allocator. If you wish to free up unused memory, use [`try_shrink_to_fit`]
/// or [`try_shrink_to`].
///
/// [`try_push`] and [`try_insert`] will never (re)allocate if the reported capacity is
/// sufficient. [`try_push`] and [`try_insert`] *will* (re)allocate if
/// <code>[len] == [capacity]</code>. That is, the reported capacity is completely
/// accurate, and can be relied on. It can even be used to manually free the memory
/// allocated by a `Vec` if desired. Bulk insertion methods *may* reallocate, even
/// when not necessary.
///
/// `Vec` does not guarantee any particular growth strategy when reallocating
/// when full, nor when [`try_reserve`] is called. The current strategy is basic
/// and it may prove desirable to use a non-constant growth factor. Whatever
/// strategy is used will of course guarantee *O*(1) amortized [`try_push`].
///
/// `try_vec![x; n]`, `try_vec![a, b, c, d]`, and [`Vec::try_with_capacity(n)`],
/// will all produce a `Vec` with exactly the requested capacity. If <code>[len]
/// == [capacity]</code>, (as is the case for the [`try_vec!`] macro), then a
/// `Vec<T>` can be converted to and from a [`Box<[T]>`][owned slice] without
/// reallocating or moving the elements.
///
/// [`Vec::try_with_capacity(n)`]: Vec::try_with_capacity
///
/// `Vec` will not specifically overwrite any data that is removed from it,
/// but also won't specifically preserve it. Its uninitialized memory is
/// scratch space that it may use however it wants. It will generally just do
/// whatever is most efficient or otherwise easy to implement. Do not rely on
/// removed data to be erased for security purposes. Even if you drop a `Vec`, its
/// buffer may simply be reused by another allocation. Even if you zero a `Vec`'s memory
/// first, that might not actually happen because the optimizer does not consider
/// this a side-effect that must be preserved. There is one case which we will
/// not break, however: using `unsafe` code to write to the excess capacity,
/// and then increasing the length to match, is always valid.
///
/// Currently, `Vec` does not guarantee the order in which elements are dropped.
/// The order has changed in the past and may change again.
///
/// [`get`]: slice::get
/// [`get_mut`]: slice::get_mut
/// [`String`]: crate::string::String
/// [`&str`]: type@str
/// [`try_shrink_to_fit`]: Vec::try_shrink_to_fit
/// [`try_shrink_to`]: Vec::try_shrink_to
/// [capacity]: Vec::capacity
/// [`capacity`]: Vec::capacity
/// [mem::size_of::\<T>]: core::mem::size_of
/// [len]: Vec::len
/// [`len`]: Vec::len
/// [`try_push`]: Vec::try_push
/// [`try_insert`]: Vec::try_insert
/// [`try_reserve`]: Vec::try_reserve
/// [`MaybeUninit`]: core::mem::MaybeUninit
/// [owned slice]: Box
pub struct Vec<T, A: Allocator = Global> {
    buf: RawVec<T, A>,
    len: usize,
}

////////////////////////////////////////////////////////////////////////////////
// Inherent methods
////////////////////////////////////////////////////////////////////////////////

impl<T> Vec<T> {
    /// Constructs a new, empty `Vec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(unused_mut)]
    /// let mut vec: Vec<i32> = Vec::new();
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Vec {
            buf: RawVec::NEW,
            len: 0,
        }
    }

    /// Constructs a new, empty `Vec<T>` with at least the specified capacity.
    ///
    /// The vector will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// minimum *capacity* specified, the vector will have a zero *length*. For
    /// an explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// If it is important to know the exact allocated capacity of a `Vec`,
    /// always use the [`capacity`] method after construction.
    ///
    /// For `Vec<T>` where `T` is a zero-sized type, there will be no allocation
    /// and the capacity will always be `usize::MAX`.
    ///
    /// [Capacity and reallocation]: #capacity-and-reallocation
    /// [`capacity`]: Vec::capacity
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut vec = Vec::with_capacity(10);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = Vec::<()>::with_capacity(10);
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// ```
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, Error> {
        Self::try_with_capacity_in(capacity, Global)
    }

    /// Convert a [`Vec<T>`] into a std `Vec<T>`.
    ///
    /// The result is allocated on the heap, using the default global allocator
    /// so this is a zero-copy operation.
    ///
    /// The memory previously occupied by this vector will be released.
    #[cfg(feature = "alloc")]
    pub fn into_std(self) -> ::rust_alloc::vec::Vec<T> {
        let (ptr, len, cap, alloc) = self.into_raw_parts_with_alloc();

        if let Ok(layout) = Layout::array::<T>(cap) {
            alloc.release(layout);
        }

        // SAFETY: All the internal invariants of this vector matches what is
        // needed to construct a rust vector, and the memory has been allocated
        // using the std `Global` allocator.
        unsafe { ::rust_alloc::vec::Vec::from_raw_parts(ptr, len, cap) }
    }
}

impl<T, A: Allocator> Vec<T, A> {
    /// Constructs a new, empty `Vec<T, A>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut vec: Vec<i32, Global> = Vec::new_in(Global);
    /// ```
    #[inline]
    pub const fn new_in(alloc: A) -> Self {
        Vec {
            buf: RawVec::new_in(alloc),
            len: 0,
        }
    }

    /// Constructs a new, empty `Vec<T, A>` with at least the specified capacity
    /// with the provided allocator.
    ///
    /// The vector will be able to hold at least `capacity` elements without
    /// reallocating. This method is allowed to allocate for more elements than
    /// `capacity`. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// minimum *capacity* specified, the vector will have a zero *length*. For
    /// an explanation of the difference between length and capacity, see
    /// *[Capacity and reallocation]*.
    ///
    /// If it is important to know the exact allocated capacity of a `Vec`,
    /// always use the [`capacity`] method after construction.
    ///
    /// For `Vec<T, A>` where `T` is a zero-sized type, there will be no
    /// allocation and the capacity will always be `usize::MAX`.
    ///
    /// [Capacity and reallocation]: #capacity-and-reallocation
    /// [`capacity`]: Vec::capacity
    ///
    /// # Errors
    ///
    /// Errors with [`Error::CapacityOverflow`] if the new capacity exceeds
    /// `isize::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut vec = Vec::try_with_capacity_in(10, Global)?;
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.try_push(i)?;
    /// }
    ///
    /// assert_eq!(vec.len(), 10);
    /// assert!(vec.capacity() >= 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.try_push(11)?;
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    ///
    /// // A vector of a zero-sized type will always over-allocate, since no
    /// // allocation is necessary
    /// let vec_units = Vec::<(), Global>::try_with_capacity_in(10, Global)?;
    /// assert_eq!(vec_units.capacity(), usize::MAX);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_with_capacity_in(capacity: usize, alloc: A) -> Result<Self, Error> {
        Ok(Vec {
            buf: RawVec::try_with_capacity_in(capacity, alloc)?,
            len: 0,
        })
    }

    /// Creates a `Vec<T, A>` directly from a pointer, a capacity, a length, and
    /// an allocator.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * `ptr` must be [*currently allocated*] via the given allocator `alloc`.
    /// * `T` needs to have the same alignment as what `ptr` was allocated with.
    ///   (`T` having a less strict alignment is not sufficient, the alignment
    ///   really needs to be equal to satisfy the [`dealloc`] requirement that
    ///   memory must be allocated and deallocated with the same layout.)
    /// * The size of `T` times the `capacity` (ie. the allocated size in bytes)
    ///   needs to be the same size as the pointer was allocated with. (Because
    ///   similar to alignment, [`dealloc`] must be called with the same layout
    ///   `size`.)
    /// * `length` needs to be less than or equal to `capacity`.
    /// * The first `length` values must be properly initialized values of type
    ///   `T`.
    /// * `capacity` needs to [*fit*] the layout size that the pointer was
    ///   allocated with.
    /// * The allocated size in bytes must be no larger than `isize::MAX`. See
    ///   the safety documentation of [`pointer::offset`].
    ///
    /// These requirements are always upheld by any `ptr` that has been
    /// allocated via `Vec<T, A>`. Other allocation sources are allowed if the
    /// invariants are upheld.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures. For example it is **not** safe to build a
    /// `Vec<u8>` from a pointer to a C `char` array with length `size_t`. It's
    /// also not safe to build one from a `Vec<u16>` and its length, because the
    /// allocator cares about the alignment, and these two types have different
    /// alignments. The buffer was allocated with alignment 2 (for `u16`), but
    /// after turning it into a `Vec<u8>` it'll be deallocated with alignment 1.
    ///
    /// The ownership of `ptr` is effectively transferred to the `Vec<T>` which
    /// may then deallocate, reallocate or change the contents of memory pointed
    /// to by the pointer at will. Ensure that nothing else uses the pointer
    /// after calling this function.
    ///
    /// [`String`]: crate::string::String
    /// [`dealloc`]: crate::alloc::Allocator::deallocate
    /// [*currently allocated*]: crate::alloc::Allocator#currently-allocated-memory
    /// [*fit*]: crate::alloc::Allocator#memory-fitting
    ///
    /// # Examples
    ///
    /// ```
    /// use std::ptr;
    /// use std::mem;
    ///
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut v = Vec::try_with_capacity_in(3, Global)?;
    /// v.try_push(1)?;
    /// v.try_push(2)?;
    /// v.try_push(3)?;
    ///
    /// // Prevent running `v`'s destructor so we are in complete control
    /// // of the allocation.
    /// let mut v = mem::ManuallyDrop::new(v);
    ///
    /// // Pull out the various important pieces of information about `v`
    /// let p = v.as_mut_ptr();
    /// let len = v.len();
    /// let cap = v.capacity();
    /// let alloc = v.allocator();
    ///
    /// unsafe {
    ///     // Overwrite memory with 4, 5, 6
    ///     for i in 0..len {
    ///         ptr::write(p.add(i), 4 + i);
    ///     }
    ///
    ///     // Put everything back together into a Vec
    ///     let rebuilt = Vec::from_raw_parts_in(p, len, cap, alloc.clone());
    ///     assert_eq!(rebuilt, [4, 5, 6]);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Using memory that was allocated elsewhere:
    ///
    /// ```rust
    /// use core::alloc::Layout;
    ///
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::{Allocator, AllocError, Global};
    ///
    /// let layout = Layout::array::<u32>(16).expect("overflow cannot happen");
    ///
    /// let vec = unsafe {
    ///     let mem = match Global.allocate(layout) {
    ///         Ok(mem) => mem.cast::<u32>().as_ptr(),
    ///         Err(AllocError) => return,
    ///     };
    ///
    ///     mem.write(1_000_000);
    ///
    ///     Vec::from_raw_parts_in(mem, 1, 16, Global)
    /// };
    ///
    /// assert_eq!(vec, &[1_000_000]);
    /// assert_eq!(vec.capacity(), 16);
    /// ```
    ///
    /// [`pointer::offset`]: primitive@pointer
    #[inline]
    pub unsafe fn from_raw_parts_in(ptr: *mut T, length: usize, capacity: usize, alloc: A) -> Self {
        unsafe {
            Vec {
                buf: RawVec::from_raw_parts_in(ptr, capacity, alloc),
                len: length,
            }
        }
    }

    /// Returns a reference to the underlying allocator.
    #[inline]
    pub fn allocator(&self) -> &A {
        self.buf.allocator()
    }

    pub(crate) fn into_raw_vec(self) -> (RawVec<T, A>, usize) {
        let me = ManuallyDrop::new(self);
        let buf = unsafe { ptr::read(&me.buf) };
        (buf, me.len)
    }

    /// Decomposes a `Vec<T>` into its raw components.
    ///
    /// Returns the raw pointer to the underlying data, the length of the vector
    /// (in elements), the allocated capacity of the data (in elements), and the
    /// allocator. These are the same arguments in the same order as the
    /// arguments to [`from_raw_parts_in`].
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `Vec`. The only way to do this is to convert
    /// the raw pointer, length, and capacity back into a `Vec` with the
    /// [`from_raw_parts_in`] function, allowing the destructor to perform the
    /// cleanup.
    ///
    /// [`from_raw_parts_in`]: Vec::from_raw_parts_in
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut v: Vec<i32> = Vec::new_in(Global);
    /// v.try_push(-1)?;
    /// v.try_push(0)?;
    /// v.try_push(1)?;
    ///
    /// let (ptr, len, cap, alloc) = v.into_raw_parts_with_alloc();
    ///
    /// let rebuilt = unsafe {
    ///     // We can now make changes to the components, such as
    ///     // transmuting the raw pointer to a compatible type.
    ///     let ptr = ptr as *mut u32;
    ///
    ///     Vec::from_raw_parts_in(ptr, len, cap, alloc)
    /// };
    ///
    /// assert_eq!(rebuilt, [4294967295, 0, 1]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn into_raw_parts_with_alloc(self) -> (*mut T, usize, usize, A) {
        let mut me = ManuallyDrop::new(self);
        let len = me.len();
        let capacity = me.capacity();
        let ptr = me.as_mut_ptr();
        let alloc = unsafe { ptr::read(me.allocator()) };
        (ptr, len, capacity, alloc)
    }

    /// Returns the total number of elements the vector can hold without
    /// reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut vec: Vec<i32> = Vec::try_with_capacity_in(10, Global)?;
    /// vec.try_push(42)?;
    /// assert!(vec.capacity() >= 10);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Tries to reserve capacity for at least `additional` more elements to be inserted
    /// in the given `Vec<T>`. The collection may reserve more space to speculatively avoid
    /// frequent reallocations. After calling `try_reserve`, capacity will be
    /// greater than or equal to `self.len() + additional` if it returns
    /// `Ok(())`. Does nothing if capacity is already sufficient. This method
    /// preserves the contents even if an error occurs.
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{Vec, Error};
    ///
    /// fn process_data(data: &[u32]) -> Result<Vec<u32>, Error> {
    ///     let mut output = Vec::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     output.try_reserve(data.len())?;
    ///
    ///     for value in data {
    ///        output.try_push(*value)?;
    ///     }
    ///
    ///     Ok(output)
    /// }
    /// # process_data(&[1, 2, 3]).expect("why is the test harness OOMing on 12 bytes?");
    /// ```
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), Error> {
        self.buf.try_reserve(self.len, additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional`
    /// elements to be inserted in the given `Vec<T>`. Unlike [`try_reserve`],
    /// this will not deliberately over-allocate to speculatively avoid frequent
    /// allocations. After calling `try_reserve_exact`, capacity will be greater
    /// than or equal to `self.len() + additional` if it returns `Ok(())`.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`try_reserve`] if future insertions are expected.
    ///
    /// [`try_reserve`]: Vec::try_reserve
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{Vec, Error};
    /// use rune::alloc::prelude::*;
    ///
    /// fn process_data(data: &[u32]) -> Result<Vec<u32>, Error> {
    ///     let mut output = Vec::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     output.try_reserve_exact(data.len())?;
    ///
    ///     // Now we know this can't OOM in the middle of our complex work
    ///     output.try_extend(data.iter().map(|&val| {
    ///         val * 2 + 5 // very complicated
    ///     }))?;
    ///
    ///     Ok(output)
    /// }
    /// # process_data(&[1, 2, 3]).expect("why is the test harness OOMing on 12 bytes?");
    /// ```
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), Error> {
        self.buf.try_reserve_exact(self.len, additional)
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the vector that there is space for a few more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut vec = Vec::try_with_capacity(10)?;
    /// vec.try_extend([1, 2, 3])?;
    /// assert!(vec.capacity() >= 10);
    /// vec.try_shrink_to_fit()?;
    /// assert!(vec.capacity() >= 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_shrink_to_fit(&mut self) -> Result<(), Error> {
        // The capacity is never less than the length, and there's nothing to do when
        // they are equal, so we can avoid the panic case in `RawVec::shrink_to_fit`
        // by only calling it with a greater capacity.
        if self.capacity() > self.len {
            self.buf.try_shrink_to_fit(self.len)?;
        }

        Ok(())
    }

    /// Shrinks the capacity of the vector with a lower bound.
    ///
    /// The capacity will remain at least as large as both the length
    /// and the supplied value.
    ///
    /// If the current capacity is less than the lower limit, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut vec = Vec::try_with_capacity(10)?;
    /// vec.try_extend([1, 2, 3])?;
    /// assert!(vec.capacity() >= 10);
    /// vec.try_shrink_to(4)?;
    /// assert!(vec.capacity() >= 4);
    /// vec.try_shrink_to(0)?;
    /// assert!(vec.capacity() >= 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_shrink_to(&mut self, min_capacity: usize) -> Result<(), Error> {
        if self.capacity() > min_capacity {
            self.buf
                .try_shrink_to_fit(cmp::max(self.len, min_capacity))?;
        }

        Ok(())
    }

    /// Converts the vector into [`Box<[T]>`][owned slice].
    ///
    /// If the vector has excess capacity, its items will be moved into a
    /// newly-allocated buffer with exactly the right capacity.
    ///
    /// [owned slice]: Box
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let v = try_vec![1, 2, 3];
    /// let slice = v.try_into_boxed_slice()?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Any excess capacity is removed:
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut vec = Vec::try_with_capacity(10)?;
    /// vec.try_extend([1, 2, 3])?;
    ///
    /// assert!(vec.capacity() >= 10);
    /// let slice = vec.try_into_boxed_slice()?;
    /// assert_eq!(Vec::from(slice).capacity(), 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_into_boxed_slice(mut self) -> Result<Box<[T], A>, Error> {
        unsafe {
            self.try_shrink_to_fit()?;
            let me = ManuallyDrop::new(self);
            let buf = ptr::read(&me.buf);
            let len = me.len();
            Ok(buf.into_box(len).assume_init())
        }
    }

    /// Shortens the vector, keeping the first `len` elements and dropping
    /// the rest.
    ///
    /// If `len` is greater than the vector's current length, this has no
    /// effect.
    ///
    /// The [`drain`] method can emulate `truncate`, but causes the excess
    /// elements to be returned instead of dropped.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// Truncating a five element vector to two elements:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3, 4, 5];
    /// vec.truncate(2);
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// No truncation occurs when `len` is greater than the vector's current
    /// length:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// vec.truncate(8);
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Truncating when `len == 0` is equivalent to calling the [`clear`]
    /// method.
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// vec.truncate(0);
    /// assert_eq!(vec, []);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`clear`]: Vec::clear
    /// [`drain`]: Vec::drain
    pub fn truncate(&mut self, len: usize) {
        // This is safe because:
        //
        // * the slice passed to `drop_in_place` is valid; the `len > self.len`
        //   case avoids creating an invalid slice, and
        // * the `len` of the vector is shrunk before calling `drop_in_place`,
        //   such that no value will be dropped twice in case `drop_in_place`
        //   were to panic once (if it panics twice, the program aborts).
        unsafe {
            // Note: It's intentional that this is `>` and not `>=`.
            //       Changing it to `>=` has negative performance
            //       implications in some cases. See #78884 for more.
            if len > self.len {
                return;
            }
            let remaining_len = self.len - len;
            let s = ptr::slice_from_raw_parts_mut(self.as_mut_ptr().add(len), remaining_len);
            self.len = len;
            ptr::drop_in_place(s);
        }
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{self, Write};
    /// use rune::alloc::try_vec;
    ///
    /// let buffer = try_vec![1, 2, 3, 5, 8];
    /// io::sink().write(buffer.as_slice()).unwrap();
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{self, Read};
    /// use rune::alloc::try_vec;
    ///
    /// let mut buffer = try_vec![0; 3];
    /// io::repeat(0b101).read_exact(buffer.as_mut_slice()).unwrap();
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self
    }

    /// Returns a raw pointer to the vector's buffer, or a dangling raw pointer
    /// valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer (non-transitively) points to
    /// is never written to (except inside an `UnsafeCell`) using this pointer or any pointer
    /// derived from it. If you need to mutate the contents of the slice, use
    /// [`as_mut_ptr`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let x = try_vec![1, 2, 4];
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`as_mut_ptr`]: Vec::as_mut_ptr
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        // We shadow the slice method of the same name to avoid going through
        // `deref`, which creates an intermediate reference.
        self.buf.ptr()
    }

    /// Returns an unsafe mutable pointer to the vector's buffer, or a dangling
    /// raw pointer valid for zero sized reads if the vector didn't allocate.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated,
    /// which would also make any pointers to it invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    ///
    /// // Allocate vector big enough for 4 elements.
    /// let size = 4;
    /// let mut x: Vec<i32> = Vec::try_with_capacity(size)?;
    /// let x_ptr = x.as_mut_ptr();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         *x_ptr.add(i) = i as i32;
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        // We shadow the slice method of the same name to avoid going through
        // `deref_mut`, which creates an intermediate reference.
        self.buf.ptr()
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type. Normally changing the length of a vector
    /// is done using one of the safe operations instead, such as
    /// [`truncate`], [`try_resize`], [`try_extend`], or [`clear`].
    ///
    /// [`truncate`]: Vec::truncate
    /// [`try_resize`]: Vec::try_resize
    /// [`try_extend`]: Extend::extend
    /// [`clear`]: Vec::clear
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity()`].
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// [`capacity()`]: Vec::capacity
    ///
    /// # Examples
    ///
    /// This method can be useful for situations in which the vector
    /// is serving as a buffer for other code, particularly over FFI:
    ///
    /// ```no_run
    /// # #![allow(dead_code)]
    /// # // This is just a minimal skeleton for the doc example;
    /// # // don't use this as a starting point for a real library.
    /// # pub(crate) struct StreamWrapper { strm: *mut std::ffi::c_void }
    /// # const Z_OK: i32 = 0;
    /// # extern "C" {
    /// #     fn deflateGetDictionary(
    /// #         strm: *mut std::ffi::c_void,
    /// #         dictionary: *mut u8,
    /// #         dictLength: *mut usize,
    /// #     ) -> i32;
    /// # }
    /// # impl StreamWrapper {
    /// pub(crate) fn get_dictionary(&self) -> Option<Vec<u8>> {
    ///     // Per the FFI method's docs, "32768 bytes is always enough".
    ///     let mut dict = Vec::with_capacity(32_768);
    ///     let mut dict_length = 0;
    ///     // SAFETY: When `deflateGetDictionary` returns `Z_OK`, it holds that:
    ///     // 1. `dict_length` elements were initialized.
    ///     // 2. `dict_length` <= the capacity (32_768)
    ///     // which makes `set_len` safe to call.
    ///     unsafe {
    ///         // Make the FFI call...
    ///         let r = deflateGetDictionary(self.strm, dict.as_mut_ptr(), &mut dict_length);
    ///         if r == Z_OK {
    ///             // ...and update the length to what was initialized.
    ///             dict.set_len(dict_length);
    ///             Some(dict)
    ///         } else {
    ///             None
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// While the following example is sound, there is a memory leak since
    /// the inner vectors were not freed prior to the `set_len` call:
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune_alloc::Error> {
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![try_vec![1, 0, 0],
    ///                        try_vec![0, 1, 0],
    ///                        try_vec![0, 0, 1]];
    /// // SAFETY:
    /// // 1. `old_len..0` is empty so no elements need to be initialized.
    /// // 2. `0 <= capacity` always holds whatever `capacity` is.
    /// unsafe {
    ///     vec.set_len(0);
    /// }
    /// # Ok(())
    /// # }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    ///
    /// Normally, here, one would use [`clear`] instead to correctly drop
    /// the contents and thus not leak memory.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is *O*(1).
    /// If you need to preserve the element order, use [`remove`] instead.
    ///
    /// [`remove`]: Vec::remove
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut v = try_vec!["foo", "bar", "baz", "qux"];
    ///
    /// assert_eq!(v.swap_remove(1), "bar");
    /// assert_eq!(v, ["foo", "qux", "baz"]);
    ///
    /// assert_eq!(v.swap_remove(0), "foo");
    /// assert_eq!(v, ["baz", "qux"]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // We replace self[index] with the last element. Note that if the
            // bounds check above succeeds there must be a last element (which
            // can be self[index] itself).
            let value = ptr::read(self.as_ptr().add(index));
            let base_ptr = self.as_mut_ptr();
            ptr::copy(base_ptr.add(len - 1), base_ptr.add(index), 1);
            self.set_len(len - 1);
            value
        }
    }

    /// Inserts an element at position `index` within the vector, shifting all
    /// elements after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// vec.try_insert(1, 4)?;
    /// assert_eq!(vec, [1, 4, 2, 3]);
    /// vec.try_insert(4, 5)?;
    /// assert_eq!(vec, [1, 4, 2, 3, 5]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_insert(&mut self, index: usize, element: T) -> Result<(), Error> {
        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        let len = self.len();

        // space for the new element
        if len == self.buf.capacity() {
            self.try_reserve(1)?;
        }

        unsafe {
            // infallible
            // The spot to put the new value
            {
                let p = self.as_mut_ptr().add(index);
                if index < len {
                    // Shift everything over to make space. (Duplicating the
                    // `index`th element into two consecutive places.)
                    ptr::copy(p, p.add(1), len - index);
                } else if index == len {
                    // No elements need shifting.
                } else {
                    assert_failed(index, len);
                }
                // Write it in, overwriting the first copy of the `index`th
                // element.
                ptr::write(p, element);
            }
            self.set_len(len + 1);
        }

        Ok(())
    }

    /// Removes and returns the element at position `index` within the vector,
    /// shifting all elements after it to the left.
    ///
    /// Note: Because this shifts over the remaining elements, it has a
    /// worst-case performance of *O*(*n*). If you don't need the order of
    /// elements to be preserved, use [`swap_remove`] instead. If you'd like to
    /// remove elements from the beginning of the `Vec`, consider using
    /// [`VecDeque::pop_front`] instead.
    ///
    /// [`swap_remove`]: crate::Vec::swap_remove
    /// [`VecDeque::pop_front`]: crate::VecDeque::pop_front
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut v = try_vec![1, 2, 3];
    /// assert_eq!(v.remove(1), 2);
    /// assert_eq!(v, [1, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = ptr::read(ptr);

                // Shift everything down to fill in that spot.
                ptr::copy(ptr.add(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            ret
        }
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3, 4];
    /// vec.retain(|&x| x % 2 == 0);
    /// assert_eq!(vec, [2, 4]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3, 4, 5];
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// vec.retain(|_| *iter.next().unwrap());
    /// assert_eq!(vec, [2, 3, 5]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.retain_mut(|elem| f(elem));
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// In other words, remove all elements `e` such that `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the
    /// original order, and preserves the order of the retained elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3, 4];
    /// vec.retain_mut(|x| if *x <= 3 {
    ///     *x += 1;
    ///     true
    /// } else {
    ///     false
    /// });
    /// assert_eq!(vec, [2, 3, 4]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        // Avoid double drop if the drop guard is not executed,
        // since we may make some holes during the process.
        unsafe { self.set_len(0) };

        // Vec: [Kept, Kept, Hole, Hole, Hole, Hole, Unchecked, Unchecked]
        //      |<-              processed len   ->| ^- next to check
        //                  |<-  deleted cnt     ->|
        //      |<-              original_len                          ->|
        // Kept: Elements which predicate returns true on.
        // Hole: Moved or dropped element slot.
        // Unchecked: Unchecked valid elements.
        //
        // This drop guard will be invoked when predicate or `drop` of element panicked.
        // It shifts unchecked elements to cover holes and `set_len` to the correct length.
        // In cases when predicate and `drop` never panick, it will be optimized out.
        struct BackshiftOnDrop<'a, T, A: Allocator> {
            v: &'a mut Vec<T, A>,
            processed_len: usize,
            deleted_cnt: usize,
            original_len: usize,
        }

        impl<T, A: Allocator> Drop for BackshiftOnDrop<'_, T, A> {
            fn drop(&mut self) {
                if self.deleted_cnt > 0 {
                    // SAFETY: Trailing unchecked items must be valid since we never touch them.
                    unsafe {
                        ptr::copy(
                            self.v.as_ptr().add(self.processed_len),
                            self.v
                                .as_mut_ptr()
                                .add(self.processed_len - self.deleted_cnt),
                            self.original_len - self.processed_len,
                        );
                    }
                }
                // SAFETY: After filling holes, all items are in contiguous memory.
                unsafe {
                    self.v.set_len(self.original_len - self.deleted_cnt);
                }
            }
        }

        let mut g = BackshiftOnDrop {
            v: self,
            processed_len: 0,
            deleted_cnt: 0,
            original_len,
        };

        fn process_loop<F, T, A: Allocator, const DELETED: bool>(
            original_len: usize,
            f: &mut F,
            g: &mut BackshiftOnDrop<'_, T, A>,
        ) where
            F: FnMut(&mut T) -> bool,
        {
            while g.processed_len != original_len {
                // SAFETY: Unchecked element must be valid.
                let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
                if !f(cur) {
                    // Advance early to avoid double drop if `drop_in_place` panicked.
                    g.processed_len += 1;
                    g.deleted_cnt += 1;
                    // SAFETY: We never touch this element again after dropped.
                    unsafe { ptr::drop_in_place(cur) };
                    // We already advanced the counter.
                    if DELETED {
                        continue;
                    } else {
                        break;
                    }
                }
                if DELETED {
                    // SAFETY: `deleted_cnt` > 0, so the hole slot must not overlap with current element.
                    // We use copy for move, and never touch this element again.
                    unsafe {
                        let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                        ptr::copy_nonoverlapping(cur, hole_slot, 1);
                    }
                }
                g.processed_len += 1;
            }
        }

        // Stage 1: Nothing was deleted.
        process_loop::<F, T, A, false>(original_len, &mut f, &mut g);

        // Stage 2: Some elements were deleted.
        process_loop::<F, T, A, true>(original_len, &mut f, &mut g);

        // All item are processed. This can be optimized to `set_len` by LLVM.
        drop(g);
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same
    /// key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![10, 20, 21, 30, 20];
    /// vec.dedup_by_key(|i| *i / 10);
    /// assert_eq!(vec, [10, 20, 30, 20]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b))
    }

    /// Removes all but the first of consecutive elements in the vector
    /// satisfying a given equality relation.
    ///
    /// The `same_bucket` function is passed references to two elements from the
    /// vector and must determine if the elements compare equal. The elements
    /// are passed in opposite order from their order in the slice, so if
    /// `same_bucket(a, b)` returns `true`, `a` is removed.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec!["foo", "bar", "Bar", "baz", "bar"];
    /// vec.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    /// assert_eq!(vec, ["foo", "bar", "baz", "bar"]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();

        if len <= 1 {
            return;
        }

        /* INVARIANT: vec.len() > read >= write > write-1 >= 0 */
        struct FillGapOnDrop<'a, T, A: Allocator> {
            /* Offset of the element we want to check if it is duplicate */
            read: usize,

            /* Offset of the place where we want to place the non-duplicate
             * when we find it. */
            write: usize,

            /* The Vec that would need correction if `same_bucket` panicked */
            vec: &'a mut Vec<T, A>,
        }

        impl<'a, T, A: Allocator> Drop for FillGapOnDrop<'a, T, A> {
            fn drop(&mut self) {
                /* This code gets executed when `same_bucket` panics */

                /* SAFETY: invariant guarantees that `read - write`
                 * and `len - read` never overflow and that the copy is always
                 * in-bounds. */
                unsafe {
                    let ptr = self.vec.as_mut_ptr();
                    let len = self.vec.len();

                    /* How many items were left when `same_bucket` panicked.
                     * Basically vec[read..].len() */
                    let items_left = len.wrapping_sub(self.read);

                    /* Pointer to first item in vec[write..write+items_left] slice */
                    let dropped_ptr = ptr.add(self.write);
                    /* Pointer to first item in vec[read..] slice */
                    let valid_ptr = ptr.add(self.read);

                    /* Copy `vec[read..]` to `vec[write..write+items_left]`.
                     * The slices can overlap, so `copy_nonoverlapping` cannot be used */
                    ptr::copy(valid_ptr, dropped_ptr, items_left);

                    /* How many items have been already dropped
                     * Basically vec[read..write].len() */
                    let dropped = self.read.wrapping_sub(self.write);

                    self.vec.set_len(len - dropped);
                }
            }
        }

        let mut gap = FillGapOnDrop {
            read: 1,
            write: 1,
            vec: self,
        };

        let ptr = gap.vec.as_mut_ptr();

        /* Drop items while going through Vec, it should be more efficient than
         * doing slice partition_dedup + truncate */

        /* SAFETY: Because of the invariant, read_ptr, prev_ptr and write_ptr
         * are always in-bounds and read_ptr never aliases prev_ptr */
        unsafe {
            while gap.read < len {
                let read_ptr = ptr.add(gap.read);
                let prev_ptr = ptr.add(gap.write.wrapping_sub(1));

                if same_bucket(&mut *read_ptr, &mut *prev_ptr) {
                    // Increase `gap.read` now since the drop may panic.
                    gap.read += 1;
                    /* We have found duplicate, drop it in-place */
                    ptr::drop_in_place(read_ptr);
                } else {
                    let write_ptr = ptr.add(gap.write);

                    /* Because `read_ptr` can be equal to `write_ptr`, we either
                     * have to use `copy` or conditional `copy_nonoverlapping`.
                     * Looks like the first option is faster. */
                    ptr::copy(read_ptr, write_ptr, 1);

                    /* We have filled that place, so go further */
                    gap.write += 1;
                    gap.read += 1;
                }
            }

            /* Technically we could let `gap` clean up with its Drop, but
             * when `same_bucket` is guaranteed to not panic, this bloats a little
             * the codegen, so we just do it manually */
            gap.vec.set_len(gap.write);
            mem::forget(gap);
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut vec: Vec<i32> = Vec::try_with_capacity_in(2, Global)?;
    /// vec.try_push(1)?;
    /// vec.try_push(2)?;
    /// vec.try_push(3)?;
    /// assert_eq!(vec, [1, 2, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_push(&mut self, value: T) -> Result<(), Error> {
        // This will panic or abort if we would allocate > isize::MAX bytes
        // or if the length increment would overflow for zero-sized types.
        if self.len == self.buf.capacity() {
            self.buf.try_reserve_for_push(self.len)?;
        }

        unsafe {
            let end = self.as_mut_ptr().add(self.len);
            ptr::write(end, value);
            self.len += 1;
        }

        Ok(())
    }

    /// Appends an element if there is sufficient spare capacity, otherwise an
    /// error is returned with the element.
    ///
    /// Unlike [`try_push`] this method will not reallocate when there's
    /// insufficient capacity. The caller should use [`try_reserve`] to ensure
    /// that there is enough capacity.
    ///
    /// [`try_push`]: Vec::try?push
    /// [`try_reserve`]: Vec::try_reserve
    ///
    /// # Examples
    ///
    /// A manual, alternative to [`TryFromIteratorIn`]:
    ///
    /// ```
    /// use rune::alloc::{Vec, Error};
    /// use rune::alloc::prelude::*;
    ///
    /// fn from_iter_fallible<T>(iter: impl Iterator<Item=T>) -> Result<Vec<T>, Error> {
    ///     let mut vec = Vec::new();
    ///
    ///     for value in iter {
    ///         if let Err(value) = vec.push_within_capacity(value) {
    ///             vec.try_reserve(1)?;
    ///             // this cannot fail, the previous line either returned or added at least 1 free slot
    ///             let _ = vec.push_within_capacity(value);
    ///         }
    ///     }
    ///
    ///     Ok(vec)
    /// }
    ///
    /// assert_eq!(from_iter_fallible(0..100), Ok(Vec::try_from_iter(0..100)?));
    /// # Ok::<_, Error>(())
    /// ```
    #[inline]
    pub fn push_within_capacity(&mut self, value: T) -> Result<(), T> {
        if self.len == self.buf.capacity() {
            return Err(value);
        }

        unsafe {
            let end = self.as_mut_ptr().add(self.len);
            ptr::write(end, value);
            self.len += 1;
        }

        Ok(())
    }

    /// Removes the last element from a vector and returns it, or [`None`] if it
    /// is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut vec = Vec::try_from_iter([1, 2, 3])?;
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(ptr::read(self.as_ptr().add(self.len())))
            }
        }
    }

    /// Moves all the elements of `other` into `self`, leaving `other` empty.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// let mut vec2 = try_vec![4, 5, 6];
    /// vec.try_append(&mut vec2)?;
    /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
    /// assert_eq!(vec2, []);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_append(&mut self, other: &mut Self) -> Result<(), Error> {
        unsafe {
            self.try_append_elements(other.as_slice() as _)?;
            other.set_len(0);
        }

        Ok(())
    }

    /// Appends elements to `self` from other buffer.
    #[inline]
    unsafe fn try_append_elements(&mut self, other: *const [T]) -> Result<(), Error> {
        let count = unsafe { (*other).len() };
        self.try_reserve(count)?;
        let len = self.len();
        unsafe { ptr::copy_nonoverlapping(other as *const T, self.as_mut_ptr().add(len), count) };
        self.len += count;
        Ok(())
    }

    /// Construct a raw iterator over the current vector
    ///
    /// # Safety
    ///
    /// The caller must ensure that any pointers returned by the iterator are
    /// not dereferenced unless the object they were constructed from is still
    /// alive.
    pub unsafe fn raw_iter(&self) -> RawIter<T> {
        RawIter::new(self)
    }

    /// Construct a raw mutable iterator over the current vector
    ///
    /// # Safety
    ///
    /// The caller must ensure that any pointers returned by the iterator are
    /// not dereferenced unless the object they were constructed from is still
    /// alive.
    ///
    /// As a mutable iterator, this also implies that *no other* mutable
    /// accesses are performed over the collection this was constructed from
    /// until the returned iterator has been dropped.
    pub unsafe fn raw_iter_mut(&mut self) -> RawIterMut<T> {
        RawIterMut::new(self)
    }

    /// Removes the specified range from the vector in bulk, returning all
    /// removed elements as an iterator. If the iterator is dropped before
    /// being fully consumed, it drops the remaining removed elements.
    ///
    /// The returned iterator keeps a mutable borrow on the vector to optimize
    /// its implementation.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the vector.
    ///
    /// # Leaking
    ///
    /// If the returned iterator goes out of scope without being dropped (due to
    /// [`mem::forget`], for example), the vector may have lost and leaked
    /// elements arbitrarily, including elements outside the range.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{try_vec, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let mut v = try_vec![1, 2, 3];
    /// let u: Vec<_> = v.drain(1..).try_collect()?;
    /// assert_eq!(v, &[1]);
    /// assert_eq!(u, &[2, 3]);
    ///
    /// // A full range clears the vector, like `clear()` does
    /// v.drain(..);
    /// assert_eq!(v, &[]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety
        //
        // When the Drain is first created, it shortens the length of
        // the source vector to make sure no uninitialized or moved-from elements
        // are accessible at all if the Drain's destructor never gets to run.
        //
        // Drain will ptr::read out the values to remove.
        // When finished, remaining tail of the vec is copied back to cover
        // the hole, and the vector length is restored to the new length.
        //
        let len = self.len();
        let Range { start, end } = slice_range(range, ..len);

        unsafe {
            // set self.vec length's to start, to be safe in case Drain is leaked
            self.set_len(start);
            let range_slice = slice::from_raw_parts(self.as_ptr().add(start), end - start);
            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                vec: NonNull::from(self),
            }
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut v = try_vec![1, 2, 3];
    /// v.clear();
    /// assert!(v.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        let elems: *mut [T] = self.as_mut_slice();

        // SAFETY:
        // - `elems` comes directly from `as_mut_slice` and is therefore valid.
        // - Setting `self.len` before calling `drop_in_place` means that,
        //   if an element's `Drop` impl panics, the vector's `Drop` impl will
        //   do nothing (leaking the rest of the elements) instead of dropping
        //   some twice.
        unsafe {
            self.len = 0;
            ptr::drop_in_place(elems);
        }
    }

    /// Returns the number of elements in the vector, also referred to as its
    /// 'length'.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut a = Vec::new_in(Global);
    ///
    /// for value in 0..3 {
    ///     a.try_push(value)?;
    /// }
    ///
    /// assert_eq!(a.len(), 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    ///
    /// let mut v = Vec::new();
    /// assert!(v.is_empty());
    ///
    /// v.try_push(1)?;
    /// assert!(!v.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Splits the collection into two at the given index.
    ///
    /// Returns a newly allocated vector containing the elements in the range
    /// `[at, len)`. After the call, the original vector will be left containing
    /// the elements `[0, at)` with its previous capacity unchanged.
    ///
    /// # Panics
    ///
    /// Panics if `at > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// let vec2 = vec.try_split_off(1)?;
    /// assert_eq!(vec, [1]);
    /// assert_eq!(vec2, [2, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use = "use `.truncate()` if you don't need the other half"]
    pub fn try_split_off(&mut self, at: usize) -> Result<Self, Error>
    where
        A: Clone,
    {
        #[cold]
        #[inline(never)]
        fn assert_failed(at: usize, len: usize) -> ! {
            panic!("`at` split index (is {at}) should be <= len (is {len})");
        }

        if at > self.len() {
            assert_failed(at, self.len());
        }

        if at == 0 {
            let new = Vec::try_with_capacity_in(self.capacity(), self.allocator().clone())?;
            // the new vector can take over the original buffer and avoid the copy
            return Ok(mem::replace(self, new));
        }

        let other_len = self.len - at;
        let mut other = Vec::try_with_capacity_in(other_len, self.allocator().clone())?;

        // Unsafely `set_len` and copy items to `other`.
        unsafe {
            self.set_len(at);
            other.set_len(other_len);
            ptr::copy_nonoverlapping(self.as_ptr().add(at), other.as_mut_ptr(), other.len());
        }

        Ok(other)
    }

    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `Vec` is extended by the
    /// difference, with each additional slot filled with the result of
    /// calling the closure `f`. The return values from `f` will end up
    /// in the `Vec` in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the `Vec` is simply truncated.
    ///
    /// This method uses a closure to create new values on every push. If
    /// you'd rather [`Clone`] a given value, use [`Vec::try_resize`]. If you
    /// want to use the [`Default`] trait to generate values, you can
    /// pass [`Default::default`] as the second argument.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 3];
    /// vec.try_resize_with(5, Default::default)?;
    /// assert_eq!(vec, [1, 2, 3, 0, 0]);
    ///
    /// let mut vec = try_vec![];
    /// let mut p = 1;
    /// vec.try_resize_with(4, || { p *= 2; p })?;
    /// assert_eq!(vec, [2, 4, 8, 16]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_resize_with<F>(&mut self, new_len: usize, f: F) -> Result<(), Error>
    where
        F: FnMut() -> T,
    {
        let len = self.len();

        if new_len > len {
            self.try_extend_trusted(iter::repeat_with(f).take(new_len - len))?;
        } else {
            self.truncate(new_len);
        }

        Ok(())
    }

    /// Consumes and leaks the `Vec`, returning a mutable reference to the contents,
    /// `&'a mut [T]`. Note that the type `T` must outlive the chosen lifetime
    /// `'a`. If the type has only static references, or none at all, then this
    /// may be chosen to be `'static`.
    ///
    /// As of Rust 1.57, this method does not reallocate or shrink the `Vec`,
    /// so the leaked allocation may include unused capacity that is not part
    /// of the returned slice.
    ///
    /// This function is mainly useful for data that lives for the remainder of
    /// the program's life. Dropping the returned reference will cause a memory
    /// leak.
    ///
    /// # Examples
    ///
    /// Simple usage:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune_alloc::Error> {
    /// let x = try_vec![1, 2, 3];
    /// let static_ref: &'static mut [usize] = x.leak();
    /// static_ref[0] += 1;
    /// assert_eq!(static_ref, &[2, 2, 3]);
    /// # Ok(())
    /// # }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    #[inline]
    pub fn leak<'a>(self) -> &'a mut [T]
    where
        A: 'a,
    {
        let mut me = ManuallyDrop::new(self);
        unsafe { slice::from_raw_parts_mut(me.as_mut_ptr(), me.len) }
    }

    /// Returns the remaining spare capacity of the vector as a slice of
    /// `MaybeUninit<T>`.
    ///
    /// The returned slice can be used to fill the vector with data (e.g. by
    /// reading from a file) before marking the data as initialized using the
    /// [`set_len`] method.
    ///
    /// [`set_len`]: Vec::set_len
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Vec;
    ///
    /// // Allocate vector big enough for 10 elements.
    /// let mut v = Vec::try_with_capacity(10)?;
    ///
    /// // Fill in the first 3 elements.
    /// let uninit = v.spare_capacity_mut();
    /// uninit[0].write(0);
    /// uninit[1].write(1);
    /// uninit[2].write(2);
    ///
    /// // Mark the first 3 elements of the vector as being initialized.
    /// unsafe {
    ///     v.set_len(3);
    /// }
    ///
    /// assert_eq!(&v, &[0, 1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        // Note:
        // This method is not implemented in terms of `split_at_spare_mut`,
        // to prevent invalidation of pointers to the buffer.
        unsafe {
            slice::from_raw_parts_mut(
                self.as_mut_ptr().add(self.len) as *mut MaybeUninit<T>,
                self.buf.capacity() - self.len,
            )
        }
    }

    /// Returns vector content as a slice of `T`, along with the remaining spare
    /// capacity of the vector as a slice of `MaybeUninit<T>`.
    ///
    /// The returned spare capacity slice can be used to fill the vector with data
    /// (e.g. by reading from a file) before marking the data as initialized using
    /// the [`set_len`] method.
    ///
    /// [`set_len`]: Vec::set_len
    ///
    /// Note that this is a low-level API, which should be used with care for
    /// optimization purposes. If you need to append data to a `Vec` you can use
    /// [`try_push`], [`try_extend`], [`try_extend_from_slice`],
    /// [`try_extend_from_within`], [`try_insert`], [`try_append`],
    /// [`try_resize`] or [`try_resize_with`], depending on your exact needs.
    ///
    /// [`try_push`]: Vec::try_push
    /// [`try_extend`]: Vec::try_extend
    /// [`try_extend_from_slice`]: Vec::try_extend_from_slice
    /// [`try_extend_from_within`]: Vec::try_extend_from_within
    /// [`try_insert`]: Vec::try_insert
    /// [`try_append`]: Vec::try_append
    /// [`try_resize`]: Vec::try_resize
    /// [`try_resize_with`]: Vec::try_resize_with
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut v = try_vec![1, 1, 2];
    ///
    /// // Reserve additional space big enough for 10 elements.
    /// v.try_reserve(10)?;
    ///
    /// let (init, uninit) = v.split_at_spare_mut();
    /// let sum = init.iter().copied().sum::<u32>();
    ///
    /// // Fill in the next 4 elements.
    /// uninit[0].write(sum);
    /// uninit[1].write(sum * 2);
    /// uninit[2].write(sum * 3);
    /// uninit[3].write(sum * 4);
    ///
    /// // Mark the 4 elements of the vector as being initialized.
    /// unsafe {
    ///     let len = v.len();
    ///     v.set_len(len + 4);
    /// }
    ///
    /// assert_eq!(&v, &[1, 1, 2, 4, 8, 12, 16]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        // SAFETY:
        // - len is ignored and so never changed
        let (init, spare, _) = unsafe { self.split_at_spare_mut_with_len() };
        (init, spare)
    }

    /// Safety: changing returned .2 (&mut usize) is considered the same as calling `.set_len(_)`.
    ///
    /// This method provides unique access to all vec parts at once in `try_extend_from_within`.
    unsafe fn split_at_spare_mut_with_len(
        &mut self,
    ) -> (&mut [T], &mut [MaybeUninit<T>], &mut usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - but the allocation extends out to `self.buf.capacity()` elements, possibly
        // uninitialized
        let spare_ptr = unsafe { ptr.add(self.len) };
        let spare_ptr = spare_ptr.cast::<MaybeUninit<T>>();
        let spare_len = self.buf.capacity() - self.len;

        // SAFETY:
        // - `ptr` is guaranteed to be valid for `self.len` elements
        // - `spare_ptr` is pointing one element past the buffer, so it doesn't overlap with `initialized`
        unsafe {
            let initialized = slice::from_raw_parts_mut(ptr, self.len);
            let spare = slice::from_raw_parts_mut(spare_ptr, spare_len);

            (initialized, spare, &mut self.len)
        }
    }

    #[inline]
    pub(crate) fn try_splice_in_place<R, I>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> Result<(), Error>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        let mut drain = self.drain(range);
        let mut iter = replace_with.into_iter();
        self::splice::splice(&mut drain, &mut iter)
    }

    // specific extend for `TrustedLen` iterators, called both by the specializations
    // and internal places where resolving specialization makes compilation slower
    fn try_extend_trusted(&mut self, iterator: impl iter::Iterator<Item = T>) -> Result<(), Error> {
        let (low, high) = iterator.size_hint();

        if let Some(additional) = high {
            debug_assert_eq!(
                low,
                additional,
                "TrustedLen iterator's size hint is not exact: {:?}",
                (low, high)
            );

            self.try_reserve(additional)?;

            unsafe {
                let ptr = self.as_mut_ptr();
                let mut local_len = SetLenOnDrop::new(&mut self.len);

                for element in iterator {
                    ptr::write(ptr.add(local_len.current_len()), element);
                    // Since the loop executes user code which can panic we have to update
                    // the length every step to correctly drop what we've written.
                    // NB can't overflow since we would have had to alloc the address space
                    local_len.increment_len(1);
                }
            }

            Ok(())
        } else {
            // Per TrustedLen contract a `None` upper bound means that the iterator length
            // truly exceeds usize::MAX, which would eventually lead to a capacity overflow anyway.
            // Since the other branch already panics eagerly (via `reserve()`) we do the same here.
            // This avoids additional codegen for a fallback code path which would eventually
            // panic anyway.
            Err(Error::CapacityOverflow)
        }
    }
}

impl<T, A: Allocator> Vec<T, A>
where
    T: TryClone,
{
    /// Resizes the `Vec` in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the `Vec` is extended by the
    /// difference, with each additional slot filled with `value`. If `new_len`
    /// is less than `len`, the `Vec` is simply truncated.
    ///
    /// This method requires `T` to implement [`Clone`], in order to be able to
    /// clone the passed value. If you need more flexibility (or want to rely on
    /// [`Default`] instead of [`Clone`]), use [`Vec::try_resize_with`]. If you
    /// only need to resize to a smaller size, use [`Vec::truncate`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec!["hello"];
    /// vec.try_resize(3, "world")?;
    /// assert_eq!(vec, ["hello", "world", "world"]);
    ///
    /// let mut vec = try_vec![1, 2, 3, 4];
    /// vec.try_resize(2, 0)?;
    /// assert_eq!(vec, [1, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_resize(&mut self, new_len: usize, value: T) -> Result<(), Error> {
        let len = self.len();

        if new_len > len {
            self.try_extend_with(new_len - len, value)?;
        } else {
            self.truncate(new_len);
        }

        Ok(())
    }

    /// Clones and appends all elements in a slice to the `Vec`.
    ///
    /// Iterates over the slice `other`, clones each element, and then appends
    /// it to this `Vec`. The `other` slice is traversed in-order.
    ///
    /// Note that this function is same as [`try_extend`] except that it is
    /// specialized to work with slices instead. If and when Rust gets
    /// specialization this function will likely be deprecated (but still
    /// available).
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1];
    /// vec.try_extend_from_slice(&[2, 3, 4]);
    /// assert_eq!(vec, [1, 2, 3, 4]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// [`try_extend`]: Vec::try_extend
    pub fn try_extend_from_slice(&mut self, other: &[T]) -> Result<(), Error> {
        try_extend_desugared(self, other.iter())
    }

    /// Copies elements from `src` range to the end of the vector.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if the end
    /// point is greater than the length of the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![0, 1, 2, 3, 4];
    ///
    /// vec.try_extend_from_within(2..);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4]);
    ///
    /// vec.try_extend_from_within(..2);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1]);
    ///
    /// vec.try_extend_from_within(4..8);
    /// assert_eq!(vec, [0, 1, 2, 3, 4, 2, 3, 4, 0, 1, 4, 2, 3, 4]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_extend_from_within<R>(&mut self, src: R) -> Result<(), Error>
    where
        R: RangeBounds<usize>,
    {
        let range = slice_range(src, ..self.len());
        self.try_reserve(range.len())?;

        // SAFETY:
        // - `slice::range` guarantees that the given range is valid for indexing self
        unsafe {
            // SAFETY:
            // - len is increased only after initializing elements
            let (this, spare, len) = self.split_at_spare_mut_with_len();

            // SAFETY:
            // - caller guarantees that src is a valid index
            let to_clone = this.get_unchecked(range);

            for (src, dst) in iter::zip(to_clone, spare) {
                dst.write(src.try_clone()?);
                *len += 1
            }
        }

        Ok(())
    }
}

impl<T, A: Allocator, const N: usize> Vec<[T; N], A> {
    /// Takes a `Vec<[T; N]>` and flattens it into a `Vec<T>`.
    ///
    /// # Panics
    ///
    /// Panics if the length of the resulting vector would overflow a `usize`.
    ///
    /// This is only possible when flattening a vector of arrays of zero-sized
    /// types, and thus tends to be irrelevant in practice. If
    /// `size_of::<T>() > 0`, this will never panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    /// assert_eq!(vec.pop(), Some([7, 8, 9]));
    ///
    /// let mut flattened = vec.into_flattened();
    /// assert_eq!(flattened.pop(), Some(6));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn into_flattened(self) -> Vec<T, A> {
        let (ptr, len, cap, alloc) = self.into_raw_parts_with_alloc();
        let (new_len, new_cap) = if T::IS_ZST {
            (len.checked_mul(N).expect("vec len overflow"), usize::MAX)
        } else {
            // SAFETY:
            // - `cap * N` cannot overflow because the allocation is already in
            // the address space.
            // - Each `[T; N]` has `N` valid elements, so there are `len * N`
            // valid elements in the allocation.
            (len.wrapping_mul(N), cap.wrapping_mul(N))
        };
        // SAFETY:
        // - `ptr` was allocated by `self`
        // - `ptr` is well-aligned because `[T; N]` has the same alignment as `T`.
        // - `new_cap` refers to the same sized allocation as `cap` because
        // `new_cap * size_of::<T>()` == `cap * size_of::<[T; N]>()`
        // - `len` <= `cap`, so `len * N` <= `cap * N`.
        unsafe { Vec::<T, A>::from_raw_parts_in(ptr.cast(), new_len, new_cap, alloc) }
    }
}

impl<T, A: Allocator> Vec<T, A>
where
    T: TryClone,
{
    /// Extend the vector by `n` clones of value.
    fn try_extend_with(&mut self, n: usize, value: T) -> Result<(), Error> {
        self.try_reserve(n)?;

        unsafe {
            let mut ptr = self.as_mut_ptr().add(self.len());
            // Use SetLenOnDrop to work around bug where compiler
            // might not realize the store through `ptr` through self.set_len()
            // don't alias.
            let mut local_len = SetLenOnDrop::new(&mut self.len);

            // Write all elements except the last one
            for _ in 1..n {
                ptr::write(ptr, value.try_clone()?);
                ptr = ptr.add(1);
                // Increment the length in every step in case clone() panics
                local_len.increment_len(1);
            }

            if n > 0 {
                // We can write the last element directly without cloning needlessly
                ptr::write(ptr, value);
                local_len.increment_len(1);
            }

            // len set by scope guard
        }

        Ok(())
    }
}

impl<T, A: Allocator> Vec<T, A>
where
    T: PartialEq,
{
    /// Removes consecutive repeated elements in the vector according to the
    /// [`PartialEq`] trait implementation.
    ///
    /// If the vector is sorted, this removes all duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut vec = try_vec![1, 2, 2, 3, 2];
    /// vec.dedup();
    /// assert_eq!(vec, [1, 2, 3, 2]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn dedup(&mut self) {
        self.dedup_by(|a, b| a == b)
    }
}

////////////////////////////////////////////////////////////////////////////////
// Common trait implementations for Vec
////////////////////////////////////////////////////////////////////////////////

impl<T, A: Allocator> ops::Deref for Vec<T, A> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }
}

impl<T, A: Allocator> ops::DerefMut for Vec<T, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }
}

impl<T, A: Allocator + Clone> TryClone for Vec<T, A>
where
    T: TryClone,
{
    fn try_clone(&self) -> Result<Self, Error> {
        let alloc = self.allocator().clone();
        crate::slice::to_vec(self, alloc)
    }
}

#[cfg(test)]
impl<T, A: Allocator + Clone> Clone for Vec<T, A>
where
    T: TryClone,
{
    fn clone(&self) -> Self {
        self.try_clone().abort()
    }
}

/// The hash of a vector is the same as that of the corresponding slice,
/// as required by the `core::borrow::Borrow` implementation.
///
/// ```
/// use std::hash::BuildHasher;
/// use rune::alloc::{try_vec, Vec};
///
/// let b = std::collections::hash_map::RandomState::new();
/// let v: Vec<u8> = try_vec![0xa8, 0x3c, 0x09];
/// let s: &[u8] = &[0xa8, 0x3c, 0x09];
/// assert_eq!(b.hash_one(v), b.hash_one(s));
/// # Ok::<_, rune::alloc::Error>(())
/// ```
impl<T: Hash, A: Allocator> Hash for Vec<T, A> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<T, I: SliceIndex<[T]>, A: Allocator> Index<I> for Vec<T, A> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T, I: SliceIndex<[T]>, A: Allocator> IndexMut<I> for Vec<T, A> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<T, A: Allocator> IntoIterator for Vec<T, A> {
    type Item = T;
    type IntoIter = IntoIter<T, A>;

    /// Creates a consuming iterator, that is, one that moves each value out of
    /// the vector (from start to end). The vector cannot be used after calling
    /// this.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let v = try_vec!["a".to_string(), "b".to_string()];
    /// let mut v_iter = v.into_iter();
    ///
    /// let first_element: Option<String> = v_iter.next();
    ///
    /// assert_eq!(first_element, Some("a".to_string()));
    /// assert_eq!(v_iter.next(), Some("b".to_string()));
    /// assert_eq!(v_iter.next(), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        const fn wrapping_byte_add<T>(this: *mut T, count: usize) -> *mut T {
            this.cast::<u8>().wrapping_add(count) as *mut T
        }

        unsafe {
            let mut me = ManuallyDrop::new(self);
            let alloc = ManuallyDrop::new(ptr::read(me.allocator()));
            let begin = me.as_mut_ptr();
            let end = if T::IS_ZST {
                wrapping_byte_add(begin, me.len())
            } else {
                begin.add(me.len()) as *const T
            };
            let cap = me.buf.capacity();
            IntoIter {
                buf: NonNull::new_unchecked(begin),
                phantom: PhantomData,
                cap,
                alloc,
                ptr: begin,
                end,
            }
        }
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a Vec<T, A> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, A: Allocator> IntoIterator for &'a mut Vec<T, A> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// leaf method to which various SpecFrom/SpecExtend implementations delegate when
// they have no further optimizations to apply
fn try_extend_desugared<'a, T, A: Allocator>(
    this: &mut Vec<T, A>,
    mut iterator: impl Iterator<Item = &'a T>,
) -> Result<(), Error>
where
    T: 'a + TryClone,
{
    // This is the case for a general iterator.
    //
    // This function should be the moral equivalent of:
    //
    //      for item in iterator {
    //          self.push(item);
    //      }
    while let Some(element) = iterator.next() {
        let len = this.len();
        if len == this.capacity() {
            let (lower, _) = iterator.size_hint();
            this.try_reserve(lower.saturating_add(1))?;
        }
        unsafe {
            ptr::write(this.as_mut_ptr().add(len), element.try_clone()?);
            // Since next() executes user code which can panic we have to bump the length
            // after each step.
            // NB can't overflow since we would have had to alloc the address space
            this.set_len(len + 1);
        }
    }

    Ok(())
}

/// Implements comparison of vectors, [lexicographically](Ord#lexicographical-comparison).
impl<T, A1, A2> PartialOrd<Vec<T, A2>> for Vec<T, A1>
where
    T: PartialOrd,
    A1: Allocator,
    A2: Allocator,
{
    #[inline]
    fn partial_cmp(&self, other: &Vec<T, A2>) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: Eq, A: Allocator> Eq for Vec<T, A> {}

/// Implements ordering of vectors, [lexicographically](Ord#lexicographical-comparison).
impl<T: Ord, A: Allocator> Ord for Vec<T, A> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(&**self, &**other)
    }
}

#[cfg(rune_nightly)]
unsafe impl<#[may_dangle] T, A: Allocator> Drop for Vec<T, A> {
    fn drop(&mut self) {
        unsafe {
            // use drop for [T]
            // use a raw slice to refer to the elements of the vector as weakest necessary type;
            // could avoid questions of validity in certain cases
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.len))
        }
        // RawVec handles deallocation
    }
}

#[cfg(not(rune_nightly))]
impl<T, A: Allocator> Drop for Vec<T, A> {
    fn drop(&mut self) {
        unsafe {
            // use drop for [T]
            // use a raw slice to refer to the elements of the vector as weakest necessary type;
            // could avoid questions of validity in certain cases
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.len))
        }
        // RawVec handles deallocation
    }
}

impl<T> Default for Vec<T> {
    /// Creates an empty `Vec<T>`.
    ///
    /// The vector will not allocate until elements are pushed onto it.
    fn default() -> Vec<T> {
        Vec::new()
    }
}

impl<T: fmt::Debug, A: Allocator> fmt::Debug for Vec<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T, A: Allocator> Borrow<[T]> for Vec<T, A> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self
    }
}

impl<T, A: Allocator> AsRef<Vec<T, A>> for Vec<T, A> {
    fn as_ref(&self) -> &Vec<T, A> {
        self
    }
}

impl<T, A: Allocator> AsMut<Vec<T, A>> for Vec<T, A> {
    fn as_mut(&mut self) -> &mut Vec<T, A> {
        self
    }
}

impl<T, A: Allocator> AsRef<[T]> for Vec<T, A> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, A: Allocator> AsMut<[T]> for Vec<T, A> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T> TryFrom<&[T]> for Vec<T>
where
    T: TryClone,
{
    type Error = Error;

    /// Converts a `&[T]` into a [`Vec<T>`].
    ///
    /// The result is fallibly allocated on the heap.
    fn try_from(values: &[T]) -> Result<Self, Error> {
        let mut out = Vec::try_with_capacity(values.len())?;

        for value in values {
            out.try_push(value.try_clone()?)?;
        }

        Ok(out)
    }
}

impl<T, const N: usize> TryFrom<[T; N]> for Vec<T> {
    type Error = Error;

    /// Converts a `[T; N]` into a [`Vec<T>`].
    ///
    /// The result is fallibly allocated on the heap.
    ///
    /// ```
    /// use rune::alloc::{vec, Vec};
    ///
    /// let a = Vec::try_from([1, 2, 3])?;
    /// let b: Vec<_> = [1, 2, 3].try_into()?;
    /// assert_eq!(a, b);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(arr: [T; N]) -> Result<Self, Error> {
        let mut out = Vec::try_with_capacity(arr.len())?;
        let arr = ManuallyDrop::new(arr);

        if !<T>::IS_ZST {
            // SAFETY: Vec::try_with_capacity ensures that there is enough capacity.
            unsafe {
                ptr::copy_nonoverlapping(arr.as_ptr(), out.as_mut_ptr(), N);
            }
        }

        unsafe {
            out.set_len(N);
        }

        Ok(out)
    }
}

#[cfg(feature = "alloc")]
impl<T> TryFrom<::rust_alloc::vec::Vec<T>> for Vec<T, Global> {
    type Error = Error;

    /// Converts a std `Vec<T>` into a [`Vec<T>`].
    ///
    /// The result is allocated on the heap.
    fn try_from(vec: ::rust_alloc::vec::Vec<T>) -> Result<Self, Error> {
        let mut vec = ManuallyDrop::new(vec);

        let ptr = vec.as_mut_ptr();
        let length = vec.len();
        let capacity = vec.capacity();

        if let Ok(layout) = Layout::array::<T>(capacity) {
            Global.take(layout)?;
        }

        // SAFETY: The layout of the vector is identical to the std vector and
        // it uses the same underlying allocator.
        unsafe { Ok(Self::from_raw_parts_in(ptr, length, capacity, Global)) }
    }
}

impl<T, A: Allocator, const N: usize> TryFrom<Vec<T, A>> for [T; N] {
    type Error = Vec<T, A>;

    /// Gets the entire contents of the `Vec<T>` as an array,
    /// if its size exactly matches that of the requested array.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// assert_eq!(try_vec![1, 2, 3].try_into(), Ok([1, 2, 3]));
    /// assert_eq!(<Vec<i32>>::new().try_into(), Ok([]));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// If the length doesn't match, the input comes back in `Err`:
    /// ```
    /// use rune::alloc::{try_vec, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let r: Result<[i32; 4], _> = (0..10).try_collect::<Vec<_>>()?.try_into();
    /// assert_eq!(r, Err(try_vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]));
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// If you're fine with just getting a prefix of the `Vec<T>`,
    /// you can call [`.truncate(N)`](Vec::truncate) first.
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut v = String::try_from("hello world")?.into_bytes();
    /// v.sort();
    /// v.truncate(2);
    /// let [a, b]: [_; 2] = v.try_into().unwrap();
    /// assert_eq!(a, b' ');
    /// assert_eq!(b, b'd');
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(mut vec: Vec<T, A>) -> Result<[T; N], Vec<T, A>> {
        if vec.len() != N {
            return Err(vec);
        }

        // SAFETY: `.set_len(0)` is always sound.
        unsafe { vec.set_len(0) };

        // SAFETY: A `Vec`'s pointer is always aligned properly, and
        // the alignment the array needs is the same as the items.
        // We checked earlier that we have sufficient items.
        // The items will not double-drop as the `set_len`
        // tells the `Vec` not to also drop them.
        let array = unsafe { ptr::read(vec.as_ptr() as *const [T; N]) };
        Ok(array)
    }
}

impl<T, A: Allocator> From<Box<[T], A>> for Vec<T, A> {
    /// Convert a boxed slice into a vector by transferring ownership of the
    /// existing heap allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{Box, Vec};
    /// use rune::alloc::try_vec;
    ///
    /// let s: Box<[i32]> = Box::try_from([10, 40, 30])?;
    /// let x: Vec<i32> = Vec::from(s);
    ///
    /// assert_eq!(x, [10, 40, 30]);
    ///
    /// let s: Box<[i32]> = try_vec![10, 40, 30].try_into_boxed_slice()?;
    /// let x: Vec<i32> = Vec::from(s);
    ///
    /// assert_eq!(x, [10, 40, 30]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn from(s: Box<[T], A>) -> Self {
        crate::slice::into_vec(s)
    }
}

impl<T, A: Allocator> TryFromIteratorIn<T, A> for Vec<T, A> {
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        let mut this = Vec::new_in(alloc);

        for value in iter {
            this.try_push(value)?;
        }

        Ok(this)
    }
}

#[cfg(test)]
impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::try_from_iter_in(iter, Global).abort()
    }
}

impl<T, A: Allocator> TryExtend<T> for Vec<T, A> {
    #[inline]
    fn try_extend<I: IntoIterator<Item = T>>(&mut self, iter: I) -> Result<(), Error> {
        <Self as SpecExtend<T, I::IntoIter>>::spec_extend(self, iter.into_iter())
    }
}

#[cfg(feature = "std")]
fn io_err(error: Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, error)
}

#[cfg(feature = "std")]
impl std::io::Write for Vec<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.try_extend_from_slice(buf).map_err(io_err)?;
        Ok(buf.len())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        let len = bufs.iter().map(|b| b.len()).sum();
        self.try_reserve(len).map_err(io_err)?;

        for buf in bufs {
            self.try_extend_from_slice(buf).map_err(io_err)?;
        }

        Ok(len)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.try_extend_from_slice(buf).map_err(io_err)?;
        Ok(())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

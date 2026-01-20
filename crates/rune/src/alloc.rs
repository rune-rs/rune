//! # The Rune core allocation and collections library
//!
//! This library provides smart pointers and collections for managing
//! heap-allocated values.
//!
//! It is a fork of the [`alloc`] and [`hashbrown`] crates with the following
//! additions:
//! * All allocations are fallible, and subject to memory limits imposed by the
//!   [`limit`] module.
//! * All colllections can be used by dynamic types, which can fallibly
//!   implement the trait they need. Such as [`Hash`] and [`Eq`] for [`HashMap`]
//!   or [`Ord`] for [`BTreeMap`]. This is accomplished using alternative
//!   functions which receive fallible closures and contexts, such as
//!   [`BTreeMap::get_mut_with`].
//!
//! [`alloc`]: https://doc.rust-lang.org/stable/alloc/
//! [`hashbrown`]: https://docs.rs/hashbrown
//!
//! ## Boxed values
//!
//! The [`Box`] type is a smart pointer type. There can only be one owner of a
//! [`Box`], and the owner can decide to mutate the contents, which live on the
//! heap.
//!
//! This type can be sent among threads efficiently as the size of a `Box` value
//! is the same as that of a pointer. Tree-like data structures are often built
//! with boxes because each node often has only one owner, the parent.
//!
//! ## Collections
//!
//! Implementations of the most common general purpose data structures are
//! defined in this library. They are re-exported through the
//! [standard collections library](../std/collections/index.html).
//!
//! ## Heap interfaces
//!
//! The [`alloc`] module defines the low-level interface to the default global
//! allocator. It is not compatible with the libc allocator API.
//!
//! [`Box`]: boxed
//! [`Cell`]: core::cell
//! [`RefCell`]: core::cell

#[doc(inline)]
pub use rune_alloc::abort;
#[doc(inline)]
pub use rune_alloc::alloc;
#[doc(inline)]
pub use rune_alloc::borrow;
#[doc(inline)]
pub use rune_alloc::clone;
#[doc(inline)]
pub use rune_alloc::fmt;
#[doc(inline)]
pub use rune_alloc::iter;
#[doc(inline)]
pub use rune_alloc::limit;
#[doc(inline)]
pub use rune_alloc::str;
#[doc(inline)]
pub use rune_alloc::sync;
#[doc(inline)]
pub use rune_alloc::{boxed, Box};
#[doc(inline)]
pub use rune_alloc::{btree_map, BTreeMap};
#[doc(inline)]
pub use rune_alloc::{btree_set, BTreeSet};
#[doc(inline)]
pub use rune_alloc::{error, Error, Result};
#[doc(inline)]
pub use rune_alloc::{hash_map, HashMap};
#[doc(inline)]
pub use rune_alloc::{hash_set, HashSet};
#[doc(inline)]
pub use rune_alloc::{string, String};
#[doc(inline)]
pub use rune_alloc::{try_format, try_vec};
#[doc(inline)]
pub use rune_alloc::{vec, Vec};
#[doc(inline)]
pub use rune_alloc::{vec_deque, VecDeque};

pub mod prelude {
    //! Prelude for common traits used in combination with this crate which
    //! matches the behavior of the std prelude.

    #[doc(inline)]
    pub use rune_alloc::prelude::*;
}

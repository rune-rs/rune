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
pub use rune_alloc::*;

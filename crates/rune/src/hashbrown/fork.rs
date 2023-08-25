#![allow(unused)]
#![allow(clippy::manual_map)]

// Copied and modified under the MIT license.
// Copyright (c) 2016 Amanieu d'Antras
//
// Imported using import_hashbrown.ps1, the below section is the only part
// copied by hand.
//
// After an import of the crate some sections might need to be modified.
//
// See: https://github.com/rust-lang/hashbrown
// The relevant fork: https://github.com/udoprog/hashbrown/tree/raw-infallible-context
// Relevant issue: https://github.com/rust-lang/hashbrown/issues/456

#[macro_use]
mod macros;
pub(crate) mod raw;
mod scopeguard;

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,

    /// The memory allocator returned an error
    AllocError {
        /// The layout of the allocation request that failed.
        layout: alloc::alloc::Layout,
    },
}

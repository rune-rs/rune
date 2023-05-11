//! Public types related to using rune in #[no_std] environments.

/// Environment that needs to be stored somewhere.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RawEnv {
    pub(crate) context: *const (),
    pub(crate) unit: *const (),
}

impl RawEnv {
    /// Initialize an empty raw environment.
    pub const fn null() -> RawEnv {
        RawEnv {
            context: core::ptr::null(),
            unit: core::ptr::null(),
        }
    }
}

macro_rules! alloc {
    ($lead:ident$($tail:tt)*) => {
        #[cfg(feature = "std")]
        #[allow(unused)]
        pub(crate) use std::$lead$($tail)*;
        #[cfg(not(feature = "std"))]
        #[allow(unused)]
        pub(crate) use alloc::$lead$($tail)*;
    }
}

#[cfg(feature = "std")]
pub(crate) use thiserror;
#[cfg(not(feature = "std"))]
pub(crate) mod thiserror;

#[cfg(feature = "std")]
pub use ::anyhow::Error;
#[cfg(not(feature = "std"))]
pub(crate) mod anyhow;
#[cfg(not(feature = "std"))]
pub use self::anyhow::Error;

alloc!(sync);
alloc!(vec);
alloc!(boxed);
alloc!(rc);
alloc!(borrow);
alloc!(string);

pub(crate) use ::core::convert;
pub(crate) use ::core::fmt;
pub(crate) use ::core::option;

pub(crate) mod prelude {
    alloc!(string::{String, ToString});
    alloc!(boxed::Box);
    alloc!(vec::Vec);
    alloc!(borrow::ToOwned);
}

#[cfg(feature = "std")]
pub(crate) mod collections {
    pub(crate) use std::collections::{btree_map, BTreeMap};
    pub(crate) use std::collections::{btree_set, BTreeSet};
    pub(crate) use std::collections::{hash_map, HashMap};
    pub(crate) use std::collections::{hash_set, HashSet};
    pub(crate) use std::collections::{vec_deque, VecDeque};
}

#[cfg(not(feature = "std"))]
pub(crate) mod collections {
    pub(crate) use alloc::collections::{btree_map, BTreeMap};
    pub(crate) use alloc::collections::{btree_set, BTreeSet};
    pub(crate) use alloc::collections::{vec_deque, VecDeque};
    pub(crate) use hashbrown::{hash_map, HashMap};
    pub(crate) use hashbrown::{hash_set, HashSet};
}

#[cfg(feature = "std")]
pub(crate) use std::process;

#[cfg(feature = "std")]
pub(crate) use std::io;

#[cfg(not(feature = "std"))]
pub(crate) mod io;

#[doc(inline)]
pub(crate) use rune_core::error;

#[cfg(not(feature = "std"))]
pub(crate) mod path;

#[cfg(feature = "std")]
pub(crate) use std::path;

#[cfg(not(feature = "std"))]
pub(crate) fn abort() -> ! {
    // TODO: introduce a hook or something.
    loop {}
}

#[cfg(feature = "std")]
pub(crate) fn abort() -> ! {
    std::process::abort()
}

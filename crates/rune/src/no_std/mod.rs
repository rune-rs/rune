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
    ($($vis:vis use $(::$tail:ident)+;)*) => {
        $(
            #[allow(unused)]
            $vis use ::rust_alloc $(::$tail)+;
        )*
    }
}

alloc! {
    pub(crate) use ::sync;
    pub(crate) use ::vec;
    pub(crate) use ::boxed;
    pub(crate) use ::rc;
    pub(crate) use ::borrow;
    pub(crate) use ::string;
}

pub(crate) mod std {
    alloc! {
        pub(crate) use ::boxed::Box;
        pub(crate) use ::vec::Vec;
        pub(crate) use ::string::String;
    }
}

pub(crate) mod prelude {
    alloc! {
        pub(crate) use ::string::String;
        pub(crate) use ::string::ToString;
        pub(crate) use ::boxed::Box;
        pub(crate) use ::vec::Vec;
        pub(crate) use ::borrow::ToOwned;
    }
}

#[allow(unused)]
pub(crate) mod collections {
    pub(crate) use ::rust_alloc::collections::{btree_map, BTreeMap};
    pub(crate) use ::rust_alloc::collections::{btree_set, BTreeSet};
    pub(crate) use ::rust_alloc::collections::{vec_deque, VecDeque};
    #[cfg(feature = "std")]
    pub(crate) use std::collections::{hash_map, HashMap};
    #[cfg(feature = "std")]
    pub(crate) use std::collections::{hash_set, HashSet};
}

#[cfg(feature = "std")]
pub(crate) use ::std::io;

#[cfg(not(feature = "std"))]
pub(crate) mod io;

#[doc(inline)]
pub(crate) use rune_core::error;

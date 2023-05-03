use core::any;

use crate::compile::Named;
use crate::hash::Hash;

pub use rune_macros::Any;

/// A trait which can be stored inside of an [AnyObj](crate::runtime::AnyObj).
///
/// We use our own marker trait that must be explicitly derived to prevent other
/// VM native types (like strings) which also implement `std::any::Any` from
/// being stored as an `AnyObj`.
///
/// This means, that only types which derive `Any` can be used inside of the VM:
///
/// ```
/// use rune::Any;
///
/// #[derive(Any)]
/// struct Npc {
///     name: String,
/// }
/// ```
pub trait Any: Named + any::Any {
    /// The type hash of the type.
    ///
    /// TODO: make const field when `TypeId::of` is const.
    fn type_hash() -> Hash;
}

// Internal any impls for useful types in the std library.

crate::__internal_impl_any!(crate::no_std::fmt::Error);
crate::__internal_impl_any!(crate::no_std::io::Error);
crate::__internal_impl_any!(crate::no_std::Error);
crate::__internal_impl_any!(core::cmp::Ordering);

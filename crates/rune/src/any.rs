use core::any;

use crate::compile::Named;
use crate::hash::Hash;

/// Macro to mark a value as external, which will implement all the appropriate
/// traits.
///
/// This is required to support the external type as a type argument in a
/// registered function.
///
/// ## `#[rune(item = <path>)]`
///
/// Specify the item prefix which contains this time.
///
/// This is required in order to calculate the correct type hash, if this is
/// omitted and the item is defined in a nested module the type hash won't match
/// the expected path hash.
///
/// ```
/// use rune::Any;
///
/// #[derive(Any)]
/// #[rune(item = ::process)]
/// struct Process {
///     /* .. */
/// }
///
/// fn install() -> Result<rune::Module, rune::ContextError> {
///     let mut module = rune::Module::with_crate("process")?;
///     module.ty::<Process>()?;
///     Ok(module)
/// }
/// ```
///
/// ## `#[rune(name = <ident>)]` attribute
///
/// The name of a type defaults to its identifiers, so `struct Foo {}` would be
/// given the name `Foo`.
///
/// This can be overrided with the `#[rune(name = <ident>)]` attribute:
///
/// ```
/// use rune::Any;
///
/// #[derive(Any)]
/// #[rune(name = Bar)]
/// struct Foo {
/// }
///
/// fn install() -> Result<rune::Module, rune::ContextError> {
///     let mut module = rune::Module::new();
///     module.ty::<Foo>()?;
///     Ok(module)
/// }
/// ```
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

crate::__internal_impl_any!(::std::fmt, core::fmt::Error);

cfg_std! {
    crate::__internal_impl_any!(::std::io, std::io::Error);
}

crate::__internal_impl_any!(::std::error, anyhow::Error);

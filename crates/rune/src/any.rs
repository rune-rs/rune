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
///     let mut module = rune::Module::with_crate("process");
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
///
/// ## `#[rune_derive(PROTOCOL, PROTOCOL = handler, ...)]` attribute
///
/// Can directly implement supported protocols via the rust Trait, i.e.,
/// `#[rune_derive(STRING_DEBUG)]` requires the type to implement
/// [`Debug`](core::fmt::Debug) and adds a handler for
/// [`Protocol::STRING_DEBUG`](crate::runtime::Protocol::STRING_DEBUG).
///
/// For unsupported protocols, or to deviate from the trait implementation,
/// a custom handler can be specified, this can either be a closure or a path
/// to a function.
///
/// ```
/// use rune::Any;
///
/// #[derive(Any, Debug)]
/// #[rune_derive(STRING_DEBUG)]
/// #[rune_derive(INDEX_GET = |it: Self, i: usize| it.0[i])]
/// struct Struct(Vec<usize>);
/// ```
///
/// ## `#[rune_functions(some_function, ...)]` attribute
/// 
/// Allows specifying functions that will be registered together with the type,
/// these need to be annotated with [`#[rune::function]`](crate::function).
///
/// ```
/// use rune::Any;
///
/// #[derive(Any)]
/// #[rune_functions(Self::first, second)]
/// struct Struct(bool, usize);
///
/// impl Struct {
///     #[rune::function]
///     fn first(self) -> bool {
///         self.0
///     }
/// }
/// 
/// #[rune::function]
/// fn second(it: Struct) -> usize {
///     it.1
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

crate::__internal_impl_any!(::std::fmt, crate::no_std::fmt::Error);
crate::__internal_impl_any!(::std::io, crate::no_std::io::Error);
crate::__internal_impl_any!(::std::error, crate::no_std::Error);

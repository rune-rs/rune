/// A trait which can be stored inside of an [AnyObj](crate::AnyObj).
///
/// We use our own marker trait that must be explicitly derived to prevent other
/// VM native types (like strings) which also implement `std::any::Any` from
/// being stored as an `AnyObj`.
///
/// This means, that only types which derive `Any` can be used inside of the VM:
///
/// ```rust
/// use runestick::Any;
///
/// #[derive(Any)]
/// struct Npc {
///     name: String,
/// }
/// ```
pub trait Any: std::any::Any {}

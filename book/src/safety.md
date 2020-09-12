# Safety

Rune is implemented in Rust, but that doesn't automatically make the language
safe (as Rust defines safety) since there are some uses of `unsafe`. In this
section we'll be documenting the pieces of the implementation which are
currently `unsafe`, rationalize, and document potential soundness holes.

## Conflicts in type hashes

GitHub issue: [https://github.com/rune-rs/rune/issues/15](https://github.com/rune-rs/rune/issues/15)

A type hash is a 64-bit hash which uniquely identifies a type in Rune. The type
hash for an external `Any` type is currently defined like this:

```rust
pub fn from_type_id(type_id: std::any::TypeId) -> Hash {
    unsafe { std::mem::transmute(type_id) }
}
```

The `transmute` is sound (ish), both are currently defined as 64-bit unsigned
integers. They both just have to be integers, signed or not of the same type.

The issue is that the type check to determine if an `Any` type is a specific
type is defined like this:

```rust
pub fn is<T>(&self) -> bool
where
    T: std::any::Any,
{
    Hash::from_type_id(std::any::TypeId::of::<T>()) == self.type_hash()
}
```

We could use `TypeId` directly here, but `TypeId`'s cannot be constructed for
types unknown to Rust, which prevent it from being used through a C ffi. A raw
[`AnyObjVtable`] has to be usable outside of Rust.

> An interesting detail is that this is actually [a soundness hole in Rust]
> right now.

In the future we might also implement a lookaside table stored in the `Unit` for
types registered in `Any`, which requires all types used to be registered
beforehand in order to detect these hash conflicts. Any dynamic types already
use such a table at the time we install modules into the [`Context`].

So the current conclusion is:
* Externally defined types (C ffi) must use properly *random* type hashes.
* The risk for the current safety issue is deemed to be low.

[`AnyObjVtable`]: https://github.com/rune-rs/rune/blob/e910fb9/crates/runestick/src/any.rs#L171
[a soundness hole in Rust]: https://github.com/rust-lang/rust/issues/10389
[`Context`]: https://docs.rs/runestick/0.6.16/runestick/struct.Context.html

## Internal `Any` type

Rune uses an [internal `Any` type].

Apart from the [hash conflict](#conflicts-in-type-hashes) documented above, the
implementation should be sound. We have an internal `Any` type instead of
relying on `Box<dyn Any>` to allow [`AnyObjVtable`] to be implementable by external
types to support external types through a C ffi.

[internal `Any` type]: https://docs.rs/runestick/0/runestick/struct.Any.html
[`AnyObjVtable`]: https://docs.rs/runestick/0/runestick/struct.AnyObjVtable.html

## `Shared<T>` and `UnsafeFromValue`

A large chunk of the `Shared<T>` container is `unsafe`. This is a container
which is behaviorally equivalent to `Rc<RefCell<T>>`.

We have this because it merges `Rc` and `RefCell` and provides the ability to
have ["owned borrows"] and the ability to unsafely decompose these into a raw
pointer and a raw guard, which is used in many implementations of
[`UnsafeFromValue`].

[`UnsafeFromValue`] is a conversion trait which is strictly used internally to
convert values into references. Its safety is documented in the trait.

["owned borrows"]: https://docs.rs/runestick/0/runestick/struct.Shared.html#method.into_ref
[`UnsafeFromValue`]: https://docs.rs/runestick/0/runestick/trait.UnsafeFromValue.html

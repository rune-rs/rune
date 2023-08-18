# Safety

Rune is implemented in Rust, but that doesn't automatically make the language
safe (as Rust defines safety) since there are some uses of `unsafe`. In this
section we'll be documenting the pieces of the implementation which are
currently `unsafe`, rationalize, and document potential soundness holes.

## Internal `Any` type

Rune uses an [internal `Any` type].

Apart from the [hash conflict](#conflicts-in-type-hashes) documented above, the
implementation should be sound. We have an internal `Any` type instead of
relying on `Box<dyn Any>` to allow [`AnyObjVtable`] to be implementable by external
types to support external types through a C ffi.

[internal `Any` type]: https://docs.rs/rune/0/rune/runtime/struct.AnyObj.html
[`AnyObjVtable`]: https://docs.rs/rune/0/rune/runtime/struct.AnyObjVtable.html

## `Shared<T>` and `UnsafeToRef` / `UnsafeToMut`

A large chunk of the `Shared<T>` container is `unsafe`. This is a container
which is behaviorally equivalent to `Rc<RefCell<T>>`.

We have this because it merges `Rc` and `RefCell` and provides the ability to
have ["owned borrows"] and the ability to unsafely decompose these into a raw
pointer and a raw guard, which is used in many implementations of
[`UnsafeToRef`] or [`UnsafeToMut`].

[`UnsafeToRef`] and [`UnsafeToMut`] are conversion traits which are strictly
used internally to convert values into references. Its safety is documented in
the trait.

["owned borrows"]: https://docs.rs/rune/0/rune/runtime/struct.Shared.html#method.into_ref
[`UnsafeToRef`]: https://docs.rs/rune/0/rune/runtime/trait.UnsafeToRef.html
[`UnsafeToMut`]: https://docs.rs/rune/0/rune/runtime/trait.UnsafeToMut.html

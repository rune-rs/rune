# Items and imports

Everything in Rune has a unique name. Every function and type. This name is what
identifies that thing, and is called its *item*. Rune performs compile time
checks to make sure that every item we try to use is actually defined.

The following are examples of items:
* `std::result::Result`
* `std::test::assert`

The first refers to the `Result` enum, and the second is the `assert` function.
They both live within their corresponding `std` module. `Result` is a bit
special even, since it's part of the *prelude*, allowing us to use it without
importing it. But what about `assert`?

If we wanted to use `assert` we would have to import it first with a `use`
declaration:

```rust,noplayground
{{#include ../../scripts/book/4_1/example_import.rn}}
```

```text
$> cargo run -- scripts/book/4_1/example_import.rn
== () (34.6µs)
```

Trying to use an item which doesn't exist will result in a compile error:

```rust,noplayground
{{#include ../../scripts/book/4_1/missing_item.rn}}
```

```text
$> cargo run -- scripts/book/4_1/missing_item.rn
error: compile error
  ┌─ scripts/book/4_1/missing_item.rn:2:15
  │
2 │     let foo = Foo::new();
  │               ^^^^^^^^^^ `Foo::new` is not a function
```

## Instance functions

Rune has something 

Instance functions are an exception to this, because Rune doesn't know the type
the instance function is being called on.

This is instead checked at runtime.

```rust,noplayground
{{#include ../../scripts/book/4_1/missing_instance_fn.rn}}
```

```text
$> cargo run -- scripts/book/4_1/missing_instance_fn.rn
error: virtual machine error
   ┌─ scripts/book/4_1/missing_instance_fn.rn:11:5
   │
11 │     foo.bar();
   │     ^^^^^^^^^ missing instance function `0xfb67fa086988a22d` for `type(0xc153807c3ddc98d7)``
```

> Note: The error is currently a bit nondescript. But in the future we will be
> able to provide better diagnostics by adding debug information.

What you're seeing above are type and function hashes. These uniquely identify
the item in the virtual machine and is the result of a deterministic computation
based on its item. So the hash for the item `Foo::new` will always be the same.

In Rust, we can calculate this hash using `Item` and `Hash::function` method:

```rust,noplayground
{{#include ../../crates/rune-testing/examples/function_hash}}
```

```text
$> cargo run --example function_hash
0xb5dc92ab43cb37d9
```

The exact implementation of the hash function is currently not defined, but will
be stabilized and documented in a future release.
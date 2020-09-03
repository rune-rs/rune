# Items and imports

Rune preforms compile time checks to make sure that every *item* in use is
defined at compile time.

An item is the fully qualified name of something, like `std::result::Result`.

Trying to use an item which doesn't exist will result in a compile time error.

```rust,noplayground
{{#include ../../scripts/book/5_0/missing_item.rn}}
```

```text
$> cargo run -- scripts/book/5_0/missing_item.rn
error: compile error
  ┌─ scripts/book/5_0/missing_item.rn:2:15
  │
2 │     let foo = Foo::new();
  │               ^^^^^^^^^^ `Foo::new` is not a function
```

Instance functions are an exception to this, because Rune doesn't know the type
the instance function is being called on.

This is instead checked at runtime.

```rust,noplayground
{{#include ../../scripts/book/5_0/missing_instance_fn.rn}}
```

```text
$> cargo run -- scripts/book/5_0/missing_instance_fn.rn
error: virtual machine error
   ┌─ scripts/book/5_0/missing_instance_fn.rn:11:5
   │
11 │     foo.bar();
   │     ^^^^^^^^^ missing instance function `0xfb67fa086988a22d` for `type(0xc153807c3ddc98d7)``
```

> Note: Unfortunately the error is currently a bit nondescript. But in the
> future we will provide the ability to provide debug information to the virtual
> machine with better diagnostics.

What you're seeing above are type and function hashes. These uniquely identify
the function or type in the virtual machine. The hash is deterministic, so the
hash for the item `Foo::new` will always be the same.

This is covered more in a future chapter.

## Imports

Like in Rust, we can import items into our scope using a `use` statement.

```rust,noplayground
{{#include ../../scripts/book/5_0/use.rn}}
```

```text
$> cargo run -- scripts/book/5_0/use.rn
== () (60.7µs)
```
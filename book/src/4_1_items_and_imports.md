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

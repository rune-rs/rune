# Items and imports

Everything in Rune has a unique name. Every function and type. This name is what
identifies that thing, and is called its *item*. Rune performs compile time
checks to make sure that every item we try to use actually exists.

The following are examples of items in Rune:

* `std::result::Result`
* `std::test::assert`

The first refers to the `Result` enum, and the second is the `assert` function.
They both live within their corresponding `std` module. `Result` is a bit
special even, since it's part of the *prelude*, allowing us to use it without
importing it. But what about `assert`?

If we wanted to use `assert` we would have to import it first with a `use`
statement:

```rune
{{#include ../../scripts/book/items_imports/example_import.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/items_imports/example_import.rn
== () (34.6µs)
```

Trying to use an item which doesn't exist results in a compile error:

```rune
{{#include ../../scripts/book/items_imports/missing_item.rn.fail}}
```

```text
$> cargo run --bin rune -- scripts/book/items_imports/missing_item.rn.fail
error: compile error
  ┌─ scripts/book/items_imports/missing_item.rn.fail:2:15
  │
2 │     let foo = Foo::new();
  │               ^^^^^^^^^^ `Foo::new` is not a function
```

Every item used in a Rune program must be known at compile time. This is one of
the guarantees a Rune scripts are required to fulfill.

# Dynamic modules

Rune has support for dynamic modules, purely defined in Rune itself. This is
done using the `mod` keyword. And the module can either be loaded from a
different file matching the name of the module or defined directly inside of the
source file.

The following is an example of an *inline* module:

```rune
{{#include ../../scripts/book/items_imports/inline_modules.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/items_imports/inline_modules.rn
== 3 (33.2µs)
```

And this is the equivalent modules loaded from the filesystem. These are three
separate files:

```rune
{{#include ../../scripts/book/items_imports/modules.rn}}
```

```rune
// file: ./foo/mod.rn
{{#include ../../scripts/book/items_imports/foo/mod.rn}}
```

```rune
// file: ./bar.rn
{{#include ../../scripts/book/items_imports/bar.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/items_imports/modules.rn
== 3 (37.5µs)
```

> Note: Rust has visibility rules (`pub`, `pub(crate)`, ...) which are not yet
> implemented in Rune. See [issue #5](https://github.com/rune-rs/rune/issues/5).

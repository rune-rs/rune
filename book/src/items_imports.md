# Items and imports

Everything in Rune has a unique name. Every function, type, and import. This
name is what identifies that thing, and is called its *item*. Rune performs
compile time checks to make sure that every item we try to use actually exists.

The following are examples of items in Rune:

* `std::result::Result` (a type)
* `std::iter::range` (a function)

The first refers to the `Result` enum, and the second is the `range` function.
They both live within their corresponding `std` module. `Result` is a bit
special even, since it's part of the *prelude*, allowing us to use it without
importing it. But what about `range`?

If we wanted to use `range` we would have to import it first with a `use`
statement:

```rune
{{#include ../../scripts/book/items_imports/example_import.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/items_imports/example_import.rn
std::iter::Range
```

Trying to use an item which doesn't exist results in a compile error:

```rune
{{#include ../../scripts/book/items_imports/missing_item.rn.fail}}
```

```text
$> cargo run --bin rune -- run scripts/book/items_imports/missing_item.rn.fail
error: compile error
  ┌─ scripts/book/items_imports/missing_item.rn.fail:2:15
  │
2 │     let foo = Foo::new();
  │               ^^^^^^^^ missing item `Foo::new`
```

Every item used in a Rune program must be known at compile time. This is one of
the static guarantees every Rune script are has to fulfill. And is one important
point where it differs from Lua or Python.

# Modules

Rune has support for modules purely defined in Rune itself. This is done using
the `mod` keyword. And the module can either be loaded from a different file
matching the name of the module or defined directly inside of the source file.

The following is an example of an *inline* module:

```rune
{{#include ../../scripts/book/items_imports/inline_modules.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/items_imports/inline_modules.rn
3
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
$> cargo run --bin rune -- run scripts/book/items_imports/modules.rn
3
```

# Disambiguating imports

Normally an item would simply be used through its local name, such as
`foo::number` above. But what happens if we need to reference a module which is
not a direct descendent of the current one or there is some ambiguation?

To this end Rune supports the following Rust keywords:
* `self` - which will resolve items from the root of the *current* module.
* `crate` - which will look up items from the entrypoint of the current project.
* `super` - which will look up items from the *parent* of the current module.

```rune
{{#include ../../scripts/book/items_imports/item_keywords.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/items_imports/item_keywords.rn
7
```

# Visibility

Every item used has to be *visible* to that item. This is governed by Runes
visibility rules, which are the following:

* An item can have inherited (empty) or a specified visibility like `pub` or
  `pub(crate)`.
* For an item to be visible, all of its parent items have to be visible.
* Items with inherited visibility are equivalent to `pub(self)`, making the item
  only visible in the module in which they are defined.

The available visibility modifiers are:
* `pub` - the item is visible from anywhere.
* `pub(crate)` - the item is visible in the same crate.
* `pub(super)` - the item is visible in the parent item only.
* `pub(self)` - the item is only visible to other items in the same module.
* `pub(in path)` - the item is only visible in the specified path. This is *not
  supported yet*.

> Note that Rune doesn't have support for crates yet, meaning `pub(crate)` and
> `pub` are currently effectively equivalent.

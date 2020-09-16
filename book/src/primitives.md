# Primitive and reference types

Primitives are values stored immediately on the stack. In Rust terminology,
these types are `Copy`, so reassigning them to different values will create
distinct *copies* of the underlying value.

The primitives available in Rune are:

* the unit `()`.
* booleans, `true` and `false`.
* bytes, like `b'\xff'`.
* characters, like `'今'`.
* integers, like `42`.
* floats, like `3.1418`.
* static strings, like `"Hello World"`.
* type hashes.

You can see that these bytes are `Copy` when assigning them to a different
variable, because a separate copy of the variable will be used.

```rune
{{#include ../../scripts/book/primitives/copy.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/primitives/copy.rn
2
1
== () (691.3µs)
```

Other types like *strings* are stored by reference on the stack. Assigning them
to a different variable will only *copy their reference*, but they still point
to the same underlying data.

```rune
{{#include ../../scripts/book/primitives/primitives.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/primitives/primitives.rn
Hello World
Hello World
== () (9.7406ms)
```
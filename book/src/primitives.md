# Primitive and reference types

Primitives are values stored immediately on the stack. In Rust terminology,
these types are `Copy`, so reassigning them to different values will create
distinct *copies* of the underlying value.

The primitives available in Rune are:

* The unit `()`.
* Booleans, `true` and `false`.
* Bytes, like `b'\xff'`.
* Characters, like `'今'`. Which are 4 byte wide characters.
* Integers, like `42`. Which are 64-bit signed integers.
* Floats, like `3.1418`. Which are 64-bit floating point numbers.
* Static strings, like `"Hello World"`.
* Type hashes.

You can see that these bytes are `Copy` when assigning them to a different
variable, because a separate copy of the value will be used automatically.

```rune
{{#include ../../scripts/book/primitives/copy.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/primitives/copy.rn
2
1
```

Other types like *strings* are stored by reference. Assigning them to a
different variable will only *copy their reference*, but they still point to the
same underlying data.

```rune
{{#include ../../scripts/book/primitives/primitives.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/primitives/primitives.rn
Hello World
Hello World
```

# Primitives and References

Primitives are values stored immediately on the stack.
In Rust terminology, these types are `Copy`, so reassigning them to different
values will create distinct copies of the underlying value.

The primitives available in rune are:

* the unit `()`.
* booleans, `true` and `false`.
* bytes, like `b'\xff'`.
* characters, like `'今'`.
* integers, like `42`.
* floats, like `3.1418`.

You can see that these bytes are `Copy`, because assigning them to a different
variable will cause a separate copy of the variable to be used.

```rust,noplaypen
{{#include ../../scripts/book/4_1/copy.rn}}
```

```text
$> cargo run -- scripts/book/4_1/copy.rn
2
1
== () (691.3µs)
```

In contrast, other types like *strings* are stored by reference on the stack.

Assigning them to a different variable will only copy the reference and increase
its reference count, but they point to the same underlying data.
As shown here:

```rust,noplaypen
{{#include ../../scripts/book/4_1/primitives.rn}}
```

```text
$> cargo run -- scripts/book/4_1/primitives.rn
Hello World
Hello World
== () (9.7406ms)
```
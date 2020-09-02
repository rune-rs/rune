# Tuples

Tuples in Rune are a fixed-size sequences of values. Similarly to a vector
tuples can contains any sequence of values. But there's no way to change the
size of a tuple.

Tuples are represented externally using the [`Tuple`] type.

```rust,noplaypen
{{#include ../../scripts/book/5_4/tuple_masquerade.rn}}
```

```text
$> cargo run -- scripts/book/5_4/tuple_masquerade.rn
("Now", "You", "See", "Me")
("Now", "You", "Don\'t", "!")
== () (38.3136ms)
```

The following is a simple example of a function returning a tuple:

```rust,noplaypen
{{#include ../../scripts/book/5_4/basic_tuples.rn}}
```

```text
$> cargo run -- scripts/book/5_4/basic_tuples.rn
(1, "test")
== () (387.6µs)
```

Tuples can also be pattern matched:

```rust,noplaypen
{{#include ../../scripts/book/5_4/tuple_patterns.rn}}
```

```text
$> cargo run -- scripts/book/5_4/tuple_patterns.rn
"the first part was a number:"
1
== () (7.7892ms)
```

[`Tuple`]: https://docs.rs/runestick/0/runestick/struct.Tuple.html
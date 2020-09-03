# Tuples

Tuples in Rune are fixed-size sequences of values. Similarly to a vector tuples
can contains any sequence of values. But there's no way to change the size of a
tuple.

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

## Using tuples from Rust

Tuples are represented externally as [primitive tuple types].

```rust,noplaypen
{{#include ../../crates/rune-testing/examples/tuple.rs}}
```

```text
$> cargo run --example tuple
(2, 4)
```

[primitive tuple types]: https://doc.rust-lang.org/std/primitive.tuple.html
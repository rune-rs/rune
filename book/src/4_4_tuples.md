# Tuples

Tuples in Rune are a fixed-size sequences of values.
Like all other containers in Rune, tuples can contains any values.

In fact, they can even change the *type* of the values stored in them, if
needed.

```rust,noplaypen
{{#include ../../scripts/book/4_4/tuple_masquerade.rn}}
```

```text
$> cargo run -- scripts/book/4_4/tuple_masquerade.rn
("Now", "You", "See", "Me")
("Now", "You", "Don\'t", "!")
== () (38.3136ms)
```

The following is a simple example of a function returning a tuple:

```rust,noplaypen
{{#include ../../scripts/book/4_4/basic_tuples.rn}}
```

```text
$> cargo run -- scripts/book/4_4/basic_tuples.rn
(1, "test")
== () (387.6µs)
```

Tuples can also be pattern matched:

```rust,noplaypen
{{#include ../../scripts/book/4_4/tuple_patterns.rn}}
```

```text
$> cargo run -- scripts/book/4_4/tuple_patterns.rn
"the first part was a number:"
1
== () (7.7892ms)
```
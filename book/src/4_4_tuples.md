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
0 = Tuple(Shared { access: fully accessible, count: 2, data: [StaticString("Now"), StaticString("You"), StaticString("See"), StaticString("Me")] })
0 = Tuple(Shared { access: fully accessible, count: 2, data: [StaticString("Now"), StaticString("You"), StaticString("Don\'t"), StaticString("!")] })
== Unit (485.6µs)
```

The following is a simple example of a function returning a tuple:

```rust,noplaypen
{{#include ../../scripts/book/4_4/basic_tuples.rn}}
```

```text
$> cargo run -- scripts/book/4_4/basic_tuples.rn
0 = Tuple(Shared { access: fully accessible, count: 1, data: [Integer(1), StaticString("test")] })
== Unit (387.6µs)
```

Tuples can also be pattern matched:

```rust,noplaypen
{{#include ../../scripts/book/4_4/tuple_patterns.rn}}
```

```text
$> cargo run -- scripts/book/4_4/tuple_patterns.rn
0 = StaticString("the first part was a number:")
1 = Integer(1)
== Unit (6.7067ms)
```
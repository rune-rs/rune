# Variables and memory

Variables in Rune are defined using the `let` keyword. In contrast to Rust, all
variables in Rune are mutable.

```rust,noplaypen
{{#include ../../scripts/book/4_2/variables.rn}}
```

```text
$> cargo run -- scripts/book/4_2/variables.rn
The value of x is: 5
The value of x is: 6
```

Rune is a memory safe language, so regardless of what you do we maintain the
same safety guarantees as safe Rust. This is accomplished in Rune through
reference counting.

## Reference counting and ownership

In Rune, [unless a value is `Copy`](5_1_primitives.md), they are reference
counted and can be used simultaneously by multiple variables. In other words
this means that they have *shared ownership*. Every variable that points to that
value therefore points to *the same instance* on the heap of that value.

```rust,noplaypen
{{#include ../../scripts/book/4_2/shared_ownership.rn}}
```

```text
$> cargo run -- scripts/book/4_2/shared_ownership.rn
1
2
== () (913.4µs)
```

This can cause issues if we call an external function expects to take ownership
of its arguments. We say that functions like these *move* their argument, and if
we try to use a variable which has been moved an error will be raised in the
virtual machine.

> Note: Below we use the `drop` function, which is a built-in function that will
> take its argument and free it.

```rust,noplaypen
{{#include ../../scripts/book/4_2/take_argument.rn}}
```

```text
$> cargo run -- scripts/book/4_2/take_argument.rn
field: 1
error: virtual machine error
  ┌─ scripts/book/4_2/take_argument.rn:6:22
  │
6 │     println(`field: {object.field}`);
  │                      ^^^^^^^^^^^^ failed to access value: cannot read, value is moved
```

If you need to, you can test if a variable is still accessible with
`is_readable` and `is_writable`.

```rust,noplaypen
{{#include ../../scripts/book/4_2/is_readable.rn}}
```

```text
$> cargo run -- scripts/book/4_2/is_readable.rn
field: 1
object is no longer readable 😢
== () (943.8µs)
```
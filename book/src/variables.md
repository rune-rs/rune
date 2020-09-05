# Variables and memory

Variables in Rune are defined using the `let` keyword. In contrast to Rust, all
variables in Rune are mutable and can be changed at any time.

```rust,noplaypen
{{#include ../../scripts/book/variables/variables.rn}}
```

```text
$> cargo run -- scripts/book/variables/variables.rn
The value of x is: 5
The value of x is: 6
```

Rune is a memory safe language. Regardless of what you write in a Rune scripts,
we maintain the same memory safety guarantees as safe Rust. This is accomplished
in Rune through reference counting.

## Reference counting and ownership

In Rune, [unless a value is `Copy`](5_1_primitives.md), they are reference
counted and can be used simultaneously at multiple locations. In other words
this means that they have *shared ownership*. Every variable that points to that
value therefore points to *the same instance* of that value.

We can see how this works by sharing and mutating one object across two
variables:

```rust,noplaypen
{{#include ../../scripts/book/variables/shared_ownership.rn}}
```

```text
$> cargo run -- scripts/book/variables/shared_ownership.rn
1
2
== () (913.4µs)
```

This can cause issues if we call an external function which expects to take
ownership of its arguments. We say that functions like these *move* their
argument, and if we try to use a variable which has been move,d an error will be
raised in the virtual machine.

> Note: Below we use the `drop` function, which is a built-in function that will
> take its argument and free it.

```rust,noplaypen
{{#include ../../scripts/book/variables/take_argument.rn}}
```

```text
$> cargo run -- scripts/book/variables/take_argument.rn
field: 1
error: virtual machine error
  ┌─ scripts/book/variables/take_argument.rn:6:22
  │
6 │     println(`field: {object.field}`);
  │                      ^^^^^^^^^^^^ failed to access value: cannot read, value is moved
```

If you need to, you can test if a variable is still accessible for reading with
`is_readable`, and for writing with `is_writable`. These are both imported in
the prelude. An object which is writable is also *movable*, and can be provided
to functions which needs to move the value, like `drop`.

```rust,noplaypen
{{#include ../../scripts/book/variables/is_readable.rn}}
```

```text
$> cargo run -- scripts/book/variables/is_readable.rn
field: 1
object is no longer readable 😢
== () (943.8µs)
```

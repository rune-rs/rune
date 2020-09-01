# Variables

Variables in Rune are defined using the `let` keyword.
In contrast to Rust, all variables in Rune are mutable and do not require a
`mut` keyword to change.

```rust,noplaypen
{{#include ../../scripts/book/3_1/variables.rn}}
```

```text
$> cargo run -- scripts/book/3_1/variables.rn
The value of x is: 5
The value of x is: 6
```

## Reference Counting and Ownership

In rune, all variables are reference counted and can be shared across multiple
variables.

This means that all variables in rune have *shared ownership*.
This means that every variable that points to an object on the stack, points to
*the same instance* of that object.

```rust,noplaypen
{{#include ../../scripts/book/3_1/shared_ownership.rn}}
```

```text
$> cargo run -- scripts/book/3_1/shared_ownership.rn
1
2
== Unit (913.4µs)
```

This can potentially cause issues if we call an external function that expects
to take ownership of its arguments.

We say that functions like these *move* their argument, and if we try to use a
variable which has been moved the virtual machine will error.

> Note: Below we use the `drop` function, which is a built-in function that will
> take its argument and free it.

```rust,noplaypen
{{#include ../../scripts/book/3_1/take_argument.rn}}
```

```text
$> cargo run -- scripts/book/3_1/take_argument.rn
field: 1
error: virtual machine error
  ┌─ scripts/book/3_1/take_argument.rn:6:22
  │
6 │     println(`field: {object.field}`);
  │                      ^^^^^^^^^^^^ failed to access value: not accessible for shared access

```

If you need to, you can test if a variable is still accessible with
`is_readable` and `is_writable`.

```rust,noplaypen
{{#include ../../scripts/book/3_1/is_readable.rn}}
```

```text
$> cargo run -- scripts/book/3_1/is_readable.rn
field: 1
it was not readable 😢
```
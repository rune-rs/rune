# Variables and memory

Variables in Rune are defined using the `let` keyword. In contrast to Rust, all
variables in Rune are mutable and can be changed at any time.

```rune
{{#include ../../scripts/book/variables/variables.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/variables/variables.rn
The value of x is: 5
The value of x is: 6
```

Rune is a memory safe language. Regardless of what you write in a Rune script,
we maintain the same memory safety guarantees as safe Rust. This is accomplished
through reference counting.

[Unless a value is `Copy`](5_1_primitives.md), they are reference counted and
can be used at multiple locations. This means that they have *shared ownership*.
Every variable that points to that value therefore points to *the same instance*
of that value. You can think of every nontrivial value being automatically
wrapped in an `Rc<RefCell<T>>` if that helps you out.

> This is not exactly what's going on. If you're interested to learn more, Rune
> uses a container called [`Shared<T>`] which is *like* an `Rc<RefCell<T>>`, but
> has a few more tricks.

We can see how this works by sharing and mutating one object across two
variables:

```rune
{{#include ../../scripts/book/variables/shared_ownership.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/variables/shared_ownership.rn
1
2
== () (913.4µs)
```

This can cause issues if we call an external function which expects to take
ownership of its arguments. We say that functions like these *move* their
argument, and if we try to use a variable which has been moved an error will be
raised in the virtual machine.

> Note: Below we use the `drop` function, which is a built-in function that will
> take its argument and free it.

```rune
{{#include ../../scripts/book/variables/take_argument.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/variables/take_argument.rn
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
to functions which need to move the value, like `drop`.

```rune
{{#include ../../scripts/book/variables/is_readable.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/variables/is_readable.rn
field: 1
object is no longer readable 😢
== () (943.8µs)
```

[`Shared<T>`]: https://docs.rs/runestick/0/runestick/struct.Shared.html

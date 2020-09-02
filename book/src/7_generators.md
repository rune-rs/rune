# Generators

Generators are a convenient method for constructing functions which are capable
of suspending themselves and their state.

The simplest use case for generators is to create a kind of iterator, whose
state is stored in the generator function.

With this, we can create a fairly efficient generator to build fibonacci
numbers.

```rust,noplaypen
{{#include ../../scripts/book/7/fib_generator.rn}}
```

```text
$> cargo run -- scripts/book/7/fib_generator.rn
0
1
1
2
3
5
8
13
21
34
55
89
144
== () (14.9441ms)
```

## Advanced generators with `GeneratorState`

Generators internally are a bit more complex than that.
The `next` function simply slates over some of that complexity to make simple
things easier to do.

The first thing to know is that `yield` itself can actually *produce* a value,
allowing the calling procedure to send values to the generator.

```rust,noplaypen
{{#include ../../scripts/book/7/send_values.rn}}
```

```text
$> cargo run -- scripts/book/7/send_values.rn
"John"
(1, 2, 3)
== () (883.2µs)
```

But wait, what happened to the first value we sent, `1`?

Well, generators don't run immediately, they need to be "warmed up" by calling
resume once.
At that point it runs the block prior to the first yield, we can see this by
instrumenting our code a little.

```rust,noplaypen
{{#include ../../scripts/book/7/bootup.rn}}
```

```text
$> cargo run -- scripts/book/7/bootup.rn
```

Ok, so we understand how to *send* values into a generator.
But how do we *receive* them?

This adds a bit of complexity, since we need to pull out `GeneratorState`.
This enum has two variants: `Yielded` and `Complete`, and represents all the
possible states a generator can suspend itself into.

```rust,noplaypen
{{#include ../../scripts/book/7/states.rn}}
```

```text
$> cargo run -- scripts/book/7/states.rn
Yielded(1)
"John"
Complete(2)
== () (712.7µs)
```

After the first call to resume, we see that the generator produced `Yielded(1)`.
This corresponds to the `yield 1` statement in the generator.

The second value we get is `Complete(2)`.
This corresponds to the *return value* of the generator.

Trying to resume the generator after this will cause the virtual machine to
error.

```rust,noplaypen
{{#include ../../scripts/book/7/error.rn}}
```

```text
$> cargo run -- scripts/book/7/error.rn
Generator { completed: false }
Yielded(1)
Complete("John")
Generator { completed: true }
error: virtual machine error
   ┌─ scripts/book/7/error.rn:11:9
   │
11 │     dbg(printer.resume(()).await);
   │         ^^^^^^^^^^^^^^^^^^^^^^^^ cannot resume generator that has completed
```
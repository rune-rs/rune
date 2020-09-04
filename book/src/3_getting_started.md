# Getting Started

The first thing you need to learn about in Rune is the `dbg` function. This is
used to "debug" values provided to it in order to understand them. Anything can
be provided to it, and it will do its best to describe it.

```rust,noplayground
{{#include ../../scripts/book/3/dbg.rn}}
```

```text
$> cargo run -- scripts/book/3/dbg.rn
[1, 2, 3]
'今'
dynamic function (at: 0x17)
native function (0x2959efc1c70)
Type(0x9aa62663879132fb)
== () (8.3679ms)
```

The default `dbg` implementation outputs information on its arguments to stdout.
But its exact behavior can differ depending on how the environment is
configured. When Rune is embedded into a larger application it might for example
be more suitable to output to a log file.

Rune also provides `print` and `println` functions which can be used to write
directly to stdout, but these cannot be relied on to be present to the same
degree as `dbg`. But for our purposes we will be using `rune-cli`, which has all
of these modules installed. This is also what was used to run the above code.

So for a more formal introduction, here is the officla Rune `"Hello World"`:

```rust,noplaypen
{{#include ../../scripts/book/3/hello_world.rn}}
```

```text
$> cargo run -- scripts/book/3/hello_world.rn
Hello World
== () (1.0864ms)
```

> Note: Something which might trip up Rust users is that the `println` is not a
> macro-like function as `println!`.

At the end of the script you see this rather odd looking line:

```text
== () (412.2µs)
```

This simply means that the script evaluated to a unit, or a `()`.
And that the execution took `412` microseconds.

> Cool Hint:
> Any function that doesn't have a return value returns a unit.

So now you know how to run Rune scripts. Well done! Let's move on to the next
chapter.

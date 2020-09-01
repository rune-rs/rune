# Getting Started

The first thing you need to learn about in Rune is the `dbg` function.
This is used to "debug" whatever values are provided to it, and can be used
by programmers in any environment to look at values in their program.

The `dbg` function output information on its arguments to stdout, but its exact
behavior is specific to the environment in which Rune is used.

When embedded into a larger application it might not be suitable to output to
stdout, so it might for example have been configured to write to a log file
instead.

Rune does also provide a `print` and `println` functions which can be used to
write directly to stdout, but these might be disabled if they're not suitable
for the environment used.

For now, lets use `println` when printing to stdout.

```rust,noplaypen
fn main() {
    println("Hello World");
}
```

You can execute this with the `rune-cli`, a commandline interface to the rune
language that comes with this project.

After each code snipped there will be a terminal showing the command used, and
its output.
Like this:

```text
$> cargo run -- scripts/hello_world.rn
Hello World
== Unit (412.2µs)
```

At the end of the script you see this rather odd looking line:

```text
== Unit (412.2µs)
```

This simply means that the script evaluated to a unit, or a `()`.
And that the execution took `412` microseconds.

> Cool Hint:
> Any function that doesn't have a return value returns a unit.

So now you know how to run Rune scripts. Well done!

Let's move on with the rest of the book.
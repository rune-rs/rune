# Getting Started

The first thing you need to learn about in Rune is the `dbg` function.
This is used to "debug" whatever values are provided to it, and can be used
by programmers in any environment to look at values in their program.

The `dbg` function *usually* outputs its arguments to stdout, but its exact
behavior is specific to the environment in which Rune is used.

When embedded into a larger application it might not be suitable to output to
stdout, so it might for example have been configured to write to a log file
instead.

Rune doesn't provide a `print` function by default, but there are modules you
can install that provide these for you if you want it to behave more like a
regular programming environment.

For now, lets use `dbg` to perform your typical "Hello World".

```rust,noplaypen
fn main() {
    dbg("Hello World");
}
```

You can execute this with the `rune` command, which is a command-line interface
to execute rune code.

It provides the closest you can get to a standard environment of rune:

```bash
$> cargo run -- scripts/hello_world.rn
    Finished dev [unoptimized + debuginfo] target(s) in 0.18s
     Running `target/debug/rune scripts/hello_world.rn`
0 = String("Hello World")
== Unit (412.2µs)
```

So now you know how to run Rune scripts. Well done!

I encourage you to follow along this book by running the provided code yourself
as with the above.

It really helps if you want to get down into it.
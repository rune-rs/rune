# Getting Started

The first thing you need to learn about in Rune is the `dbg` function. This is
used to "debug" values provided to it in order to understand them. Anything can
be provided to it, and it will do its best to describe it.

```rune
{{#include ../../scripts/book/getting_started/dbg.rn}}
```
> **Note**: by convention Rune uses files ending in .rn.

```text
$> cargo run --bin rune -- run scripts/book/getting_started/dbg.rn
[1, 2, 3]
'今'
dynamic function (at: 0x1a)
native function (0x1bd03b8ee40)
dynamic function (at: 0x17)
```

The default `dbg` implementation outputs information on its arguments to stdout.
But its exact behavior can differ depending on how the environment is
configured. When Rune is embedded into a larger application it might for example
be more suitable to output to a log file.

Rune also provides `print!` and `println!` macros which can be used to format
directly to stdout, but these cannot be relied on to be present to the same
degree as `dbg`. However for our purposes we will be using `rune-cli`, which has
all of these modules installed. This is also what was used to run the above
code.

So for a more formal introduction, here is the official Rune `"Hello World"`:

```rune
{{#include ../../scripts/book/getting_started/hello_world.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/getting_started/hello_world.rn
Hello World
```

So now you know how to run Rune scripts. Well done! Let's move on to the next
chapter.

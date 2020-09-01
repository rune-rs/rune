# Functions

Functions are truly pervasive when it comes to programming.
They encapsulate a piece of functionality and provides a contract of how they
work that can be relied on to build more complex programs.

In Rune, functions are declared with the `fn` keyword.
You've already seen one which is used in every example, `main`.
This is not a special function, but is simply what the Rune cli looks for when
deciding what to execute.

```rust,noplaypen
{{#include ../../scripts/book/4_5/main_function.rn}}
```

```text
$> cargo run -- scripts/book/4_5/main_function.rn
Hello World
== () (277.8µs)
```

In Rune, you don't have to specify the return type of a function.
Given that Rune is a dynamic programming language, this allows a function to
return anything.
Every completely distinct types.

```rust,noplaypen
{{#include ../../scripts/book/4_5/return_value.rn}}
```

```text
$> cargo run -- scripts/book/4_5/return_value.rn
Hello
1
== () (8.437ms)
```

Depending on who you talk to, this is either the best things since sliced bread
or quite scary.
It allows for a larger ability to express a program, but at the same time it can
be harder to reason on what your program will do.
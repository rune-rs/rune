# Control Flow

Rune supports a number of control flow expressions. We will be dedicating this
section to describe the most common ones.

## `return` expression

In the previous section we talked about functions. And one of the primary things
a function does is return things. The `return` expression allows for returning
from the current function. If used without an argument, the function will return
a unit `()`.

The last statement in a function is known as an *implicit return*, and will be
what the function returns by default unless a `return` is specified.

```rune
{{#include ../../scripts/book/control_flow/numbers_game.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/control_flow/numbers_game.rn
less than one
something else
== () (3.8608ms)
```

## `if` expressions

If expressions allow you to provide a condition with one or more code branches.
If the condition is `true`, the provided block of code will run.

```rune
{{#include ../../scripts/book/control_flow/conditional.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/control_flow/conditional.rn
The number *is* smaller than 5
== () (5.108ms)
```

Optionally, we can add another branch under `else`, which will execute in case
the condition is false.

```rune
{{#include ../../scripts/book/control_flow/conditional_else.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/control_flow/conditional_else.rn
the number is smaller than 5
== () (196.1µs)
```

We can also add an arbitrary number of `else if` branches, which allow us to
specify many different conditions.

```rune
{{#include ../../scripts/book/control_flow/conditional_else_ifs.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/control_flow/conditional_else_ifs.rn
the number is smaller than 5
== () (227.9µs)
```

Do note however that if you have *many* conditions, it might be cleaner to use
a `match`.

This will be covered in a later section, but here is a sneak peek:

```rune
{{#include ../../scripts/book/control_flow/first_match.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/control_flow/first_match.rn
the number is smaller than 5
== () (124.2µs)
```

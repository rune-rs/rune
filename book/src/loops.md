# Loops

Loops are a fundamental building block common to many programming languages.
This is no exception in Rune.
Loops allow you to execute a block of code until a specific condition is
reached, which can be a powerful tool for accomplishing programming tasks.

## `break` Keyword

Every loop documented in this section can be *terminated early* using the
`break` keyword.

When Rune encounters a break, it will immediately jump out of the loop it is
currently in and continue running right after it.

```rune
{{#include ../../scripts/book/loops/while_loop.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/loops/while_loop.rn
The value is 50
== () (501.1µs)
```

## `loop` Expressions

The `loop` keyword builds the most fundamental form of loop in Rune.
One that repeats unconditionally forever, until it is exited using another
control flow operator like a `break` or a `return`.

```rune
{{#include ../../scripts/book/loops/loop_forever.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/loops/loop_forever.rn
Hello forever!
Hello forever!
Hello forever!
...
```

> Hint: If you want this one to end, you're gonna have to kill it with `CTRL+C`.

We're also using an asynchronous function called `delay_for` above to avoid
spamming our terminals too much.
Well talk more about these in a later section.

When broken out of, loops produce the value provided as an argument to the
`break` keyword.
By default, this is simply a unit `()`.

```rune
{{#include ../../scripts/book/loops/loop_break.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/loops/loop_break.rn
The final count is: 11
== () (281.5µs)
```

# Functions and closures

We've gone over functions before, and while incredibly useful there's a few more
tricks worth mentioning.

We'll also be talking about closures, an anonymous function with the ability to
*close over* its environment, allowing the function to use and manipulate things
from its environment.

## Function pointers

Every function can be converted into a function pointer simply by referencing
its name without calling it.

This allows for some really neat tricks, like passing in a function which
represents the operation you want another function to use.

```rust,noplaypen
{{#include ../../scripts/book/8/function_pointers.rn}}
```

```text
$> cargo run -- scripts/book/8/function_pointers.rn
Result: 3
Result: -1
== Unit (5.4354ms)
```
# Variables

Variables in Rune are defined using the `let` keyword.

In contrast to Rust, all variables in Rune are mutable.

```rune
let x = 5;
dbg(`The value of x is: {x}`);
x = 6;
dbg(`The value of x is: {x}`);
```
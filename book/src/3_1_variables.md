# Variables

Variables in Rune are defined using the `let` keyword.
In contrast to Rust, all variables in Rune are mutable and do not require a
`mut` keyword to change.

```rune
fn main() {
    let x = 5;
    dbg(`The value of x is: {x}`);
    x = 6;
    dbg(`The value of x is: {x}`);
}
```

This would output:

```text
0 = String("The value of x is: 5")
0 = String("The value of x is: 6")
```
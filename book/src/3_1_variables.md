# Variables

Variables in Rune are defined using the `let` keyword.

In contrast to Rust, all variables in Rune are mutable.

```rune
let x = 5;
dbg("The value of x is:", x);
x = 6;
dbg("The value of x is:", x);
```

## References

Rune supports references to variables.
These are created with the `&` keyword.

Rune makes no distinction between mutable or immutable references.
Access is instead checked at runtime.

```rune
let x = "John";
let y = &x;
dbg("The value of x is:", *y);
drop(x);
dbg("The value of x is:", *y);
```

To guarantee that references aren't stale, there are two rules:
* References cannot be returne from functions.
* A live reference cannot be updated to point at a stack location which is
  higher on the stack than when it was created.

These two rules ensure that references cannot become stale, and point towards an
undefined stack location.

So the following program will not compile, because it breaks the first rule:

```rune
fn function(n) {
    let a = &n;
    let b = 5;
    [a, &b]
}
```

```text
error: compile error
  ┌─ .\scripts\book\3_1_return_references.rn:1:16
  │
1 │   fn function(n) {
  │ ╭────────────────'
2 │ │     let a = &n;
  │ │             -- reference created here
3 │ │     let b = 5;
4 │ │     [a, &b]
  │ │     ^^^^^^^
  │ │     │
  │ │     compile error
  │ │     cannot return locally created references
5 │ │ }
  │ ╰─' block returned from
```

If we were to allow it, the returned pointer would point to a stack location
which is *higher* than the function that called it.

This is an example of a function that breaks the second rule:

```rune
fn function(n) {
    let local = 2;
    *n = &local;
}
```

```text
error: runtime error
  ┌─ .\scripts\book\3_1_return_references2.rn:3:5
  │
3 │     *n = &local;
  │     ^^^^^^^^^^^ pointer cannot be changed to point to a lower stack address `2 > 0`
```

Similary if we were to allow this program to run, after the function returns the
reference would point to an invalid location.
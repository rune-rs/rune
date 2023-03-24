# Closures

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

```rune
{{#include ../../scripts/book/closures/function_pointers.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/closures/function_pointers.rn
Result: 3
Result: -1
```

## Closures

Closures are anonymous functions which closes over their environment.
This means that they capture any variables used inside of the closure, allowing
them to be used when the function is being called.

```rune
{{#include ../../scripts/book/closures/basic_closure.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/closures/basic_closure.rn
Result: 4
Result: 3
```

> Hint: Closures which do not capture their environment are *identical* in
> representation to a function.

# Functions outside of the Vm

Now things get *really* interesting.
Runestick, the virtual machine driving Rune, has support for passing function
pointers out of the virtual machine using the `Function` type.

This allows you to write code that takes a function constructed in Rune, and use
it for something else.

```rust,noplaypen
{{#include ../../examples/examples/rune_function.rs}}
```

```text
$> cargo run --example rune_function
4
8
```

Note that these functions by necessity have to capture their entire context and
can take up quite a bit of space if you keep them around while cycling many
contexts or units.

Values used in a closure can also be moved into it using the `move` keyword,
guaranteeing that no one else can use it afterwards. An attempt to do so will
cause a compile error.

```rune
{{#include ../../scripts/book/closures/closure_move.rn.fail}}
```

```text
$> cargo run --bin rune -- run scripts/book/closures/closure_move.rn.fail
error: compile error
  ┌─ scripts/book/closures/closure_move.rn.fail:7:33
  │
7 │     println!("Result: {}", work(move |a, b| n + a + b));
  │                                 --------------------- moved here
8 │     assert!(!is_readable(n));
  │                          ^ variable moved
```

> Moving indiscriminately applies to types which in principle could be copied
> (like integers). We simply don't have the necessary type information available
> right now to make that decision. If you know that the value can be copied and
> you want to do so: assign it to a separate variable.

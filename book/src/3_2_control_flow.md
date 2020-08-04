# Control Flow

Rune supports your typical forms of control flow.

## `return` Keyword

The `return` keyword allows for returning from the current function.
If specified without an argument, the function will return a unit `()`.

The last statement in a function is known as an *implicit return*, and will be
what the function returns by default unless a `return` is specified.

```rust,noplaypen
fn foo(n) {
    if n < 1 {
        return "less than one";
    }

    "something else"
}

fn main() {
    dbg(foo(0)); // => outputs: "less than one"
    dbg(foo(10)); // => outputs: "something else"
}
```

## `if` Expressions

If expressions allow you to provide a condition with one or more code branches.
If the condition is `true`, the provided block of code will run.

```rust,noplaypen
fn main() {
    let number = 3;

    if number < 5 {
        dbg("the number is smaller than 5");
    }
}
```

Optionally, we can add another branch under `else`, which will execute in case
the condition is false.

```rust,noplaypen
fn main() {
    let number = 3;

    if number < 5 {
        dbg("the number is smaller than 5");
    } else {
        dbg("the number is 5 or bigger");
    }
}
```

We can also add an arbitrary number of `else if` branches, which allow us to
specify many different conditions.

```rust,noplaypen
fn main() {
    let number = 3;

    if number < 5 {
        dbg("the number is smaller than 5");
    } else if number == 5 {
        dbg("the number is exactly 5");
    } else {
        dbg("the number is bigger than 5");
    }
}
```

Do note however that if you have *many* conditions, it might be cleaner to use
a `match`.

This will be covered in a later section, but here is a sneak peek:

```rust,noplaypen
fn main() {
    let number = 3;

    match number {
        n if n < 5 => {
            dbg("the number is smaller than 5");
        }
        5 => {
            dbg("the number is exactly 5");
        }
        n => {
            dbg("the number is bigger than 5");
        }
    }
}
```
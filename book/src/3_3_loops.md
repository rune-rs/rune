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

```rust,noplaypen
fn main() {
    let value = 0;

    while value < 100 {
        if value >= 50 {
            break;
        }

        value = value + 1;
    }

    dbg("The value is " + value); // => The value is 50
}
```

## `loop` Expressions

The `loop` keywords builds the most fundamental form of loop in Rune.
One that repeats unconditionally forever, until it is exited using another
control flow operator like a `break` or a `return`.

```rust,noplaypen
fn main() {
    loop {
        dbg("Hello forever!");
    }
}
```

When broken out of, loops produce the value provided as an argument to the
`break` keyword.
By default, this is simply a unit `()`.

```rust,noplaypen
fn main() {
    let counter = 0;

    let total = loop {
        counter = counter + 1;

        if counter > 10 {
            break counter;
        }
    };

    dbg("The final count is: " + total);
}
```
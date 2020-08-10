# Tuples

Tuples in Rune are a fixed-size collection of values.

The following is a simple example of a function returning a tuple:

```rust,noplaypen
fn foo() {
    (1, "test")
}

fn main() {
    dbg(foo());
}
```

Tuples can also be pattern matched:

```rust,noplaypen
fn main() {
    match ("test", 1) {
        ("test", n) => {
            dbg("the first part was a number:", n);
        }
        _ => {
            dbg("matched something we did not understand");
        }
    }
}
```
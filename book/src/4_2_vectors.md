# Vectors

A vector is a native data structure of Rune which is a dynamic list of values.
A vector isn't typed, and can store *any* rune values.

```rust,noplaypen
fn main() {
    let values = Vec::new();
    values.push("Hello");
    values.push(42);

    while let Some(v) = values.pop() {
        dbg(v);
    }
}
```

Which would give:

```text
0 = Integer(42)
0 = StaticString("Hello")
```

# Primitives

Primitives are values stored immediately on the stack.
In Rust terminology, these types are `Copy`, so reassigning them to different
values will create distinct copies of the underlying value.

The primitives available in rune are:

* the unit `()`.
* booleans, `true` and `false`.
* bytes, like `b'\xff'`.
* characters, like `'今'`.
* integers, like `42`.
* floats, like `3.1418`.

Any other types are stored by reference on the stack.
Assigning them to a different variable will copy the reference, but they point
to the same data.

```rust,noplaypen
fn main() {
    let a = String::new();
    a.push_str("Hello World");
    let b = a;
    dbg(a, b);
}
```

Running this will output:

```text
0 = String("Hello World")
1 = String("Hello World")
```

So both `a` and `b` point to the same data.
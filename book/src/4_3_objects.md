# Objects

Objects are anonymous maps with arbitrary string keys.

```rust,noplaypen
fn main() {
    let values = #{};
    values["first"] = "bar";
    values["second"] = 42;

    dbg(values);
}
```

This would produce:

```text
0 = Object({"first": StaticString("bar"), "second": Integer(42)})
```
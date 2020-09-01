# Try operator

The try operator (`?`) is a control flow operator which causes a function to
return early in case the value being tried over has a certain value.

For `Option`, this causes the function to return if it has the `Option::None`
variant.

```rust,noplaypen
{{#include ../../scripts/book/6_1/basic_try.rn}}
```

```text
$> cargo run -- scripts/book/6_1/basic_try.rn
Result: 2, 1
== Unit (7.4912ms)
```
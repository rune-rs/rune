# Try operator

The try operator (`?`) is a control flow operator which causes a function to
return early in case the value being tried over has a certain value.

For `Option`, this causes the function to return if it has the `Option::None`
variant.

```rune
{{#include ../../scripts/book/try_operator/basic_try.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/try_operator/basic_try.rn
Result: 2, 1
== () (7.4912ms)
```
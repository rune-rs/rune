# Closures

Closures are anonymous functions which closes over their environment.
This means that they capture any variables used inside of the closure, allowing
them to be used when the function is being called.

```rust,noplaypen
{{#include ../../scripts/8/basic_closure.rn}}
```

```text
$> cargo run -- scripts/book/8/basic_closure.rn
Result: 4
Result: 3
== Unit (5.4354ms)
```
# Streams

Streams are the asynchronous version of [Generators](./7_generators.md).

They have identical `next` and `resume` protocols, but we are now allowed to use
asynchronous functions inside of the generator.

```rust,noplayground
{{#include ../../scripts/book/10/basic_stream.rn}}
```

```text
$> cargo run -- scripts/book/10/basic_stream.rn
== () (5.4354ms)
```

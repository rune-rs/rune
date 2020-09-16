# Streams

Streams are the asynchronous version of [Generators](./7_generators.md).

They have almost identical `next` and `resume` functions, but each must be used
with `.await`, and we are now allowed to use asynchronous functions inside of
the generator.

```rune
{{#include ../../scripts/book/streams/basic_stream.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/streams/basic_stream.rn
200 OK
200 OK
== () (754.3946ms)
```

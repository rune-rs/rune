# Objects

Objects are anonymous maps, which support defining and using arbitrary string
keys.

```rune
{{#include ../../scripts/book/objects/objects.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/objects/objects.rn
"bar"
42
key did not exist
("second", 42)
("first", "bar")
== () (3.3527ms)
```

These are useful because they allow their data to be specified dynamically,
which is exactly the same use case as storing unknown JSON.

One of the biggest motivations for Rune to have anonymous objects is so that
we can natively handle data with unknown structure.

```rune
{{#include ../../scripts/book/objects/json.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/objects/json.rn
9c4bdaf194410d8b2f5d7f9f52eb3e64709d3414
06419f2580e7a18838f483321055fc06c0d75c4c
cba225dad143779a0a9543cfb05cde9710083af5
15133745237c014ff8bae53d8ff8f3c137c732c7
39ac97ab4ebe26118e807eb91c7656ab95b1fcac
3f6310eeeaca22d0373cc11d8b34d346bd12a364
== () (331.3324ms)
```

## Using objects from Rust

Objects are represented externally as the [`Object`] type alias. The keys are
always strings, but its value must be specified as the sole type parameter.
Note that the dynamic [`Value`] can be used if the type is unknown.

```rust,noplaypen
{{#include ../../crates/rune/examples/object.rs}}
```

```text
$> cargo run --example object
42
Some("World")
```

[`Object`]: https://docs.rs/runestick/0/runestick/type.Object.html
[`Value`]: https://docs.rs/runestick/0/runestick/enum.Value.html

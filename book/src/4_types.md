# Types

Types in Rune are identified uniquely by their *path*.
A path is a scope-separated identifier, like `std::float`.
This identifies a type object.

These can be used to perform basic type checking, like this:

```rust,noplaypen
{{#include ../../scripts/book/4/types.rn}}
```

Conversely, the type check would fail if it's not valid:

```text
error: virtual machine error
  ┌─ .\scripts\book\4_bad_type_check.rn:4:5
  │
4 │     assert(["hello", "world"] is String, "vectors should be strings");
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │     │
  │     virtual machine error
  │     assertion failed: vectors should be strings
  │     error in user-defined function
```

So this allows us to determine which type is which and act accordingly:

```rust,noplaypen
{{#include ../../scripts/book/4/type_check.rn}}
```

```text
$> cargo run -- scripts/book/4/type_check.rn
n is a String
n is a vector
n is unknown
== Unit (1.0544ms)
```

A tighter way to accomplish this could be by using pattern matching:

```rust,noplaypen
{{#include ../../scripts/book/4/type_check_patterns.rn}}
```

```text
$> cargo run -- scripts/book/4/type_check.rn
n is a String
n is a vector
n is unknown
== Unit (1.0544ms)
```
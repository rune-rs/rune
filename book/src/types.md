# Rune types

Types in Rune are identified uniquely by their *item*. An item path is a
scope-separated identifier, like `std::float`. This particular item identifies
a type.

These items can be used to perform basic type checking using the `is` and `is
not` operations, like this:

```rune
{{#include ../../scripts/book/types/types.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/types/types.rn
== () (120µs)
```

Conversely, the type check would fail if you're providing a value which is not
of that type.

```rune
{{#include ../../scripts/book/types/bad_type_check.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/types/bad_type_check.rn
error: virtual machine error
  ┌─ scripts/book/types/bad_type_check.rn:4:5
  │
4 │     assert(["hello", "world"] is String, "vectors should be strings");
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ panicked `assertion failed `vectors should be strings``
```

This gives us insight at runtime which type is which, and allows Rune scripts to
make decisions depending on what type a value has.

```rune
{{#include ../../scripts/book/types/type_check.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/types/type_check.rn
n is a String
n is a vector
n is unknown
== () (1.0544ms)
```

A tighter way to accomplish this would be by using pattern matching, a mechanism
especially suited for many conditional branches. Especially when the branches
are different types or variants in an enum.

```rune
{{#include ../../scripts/book/types/type_check_patterns.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/types/type_check_patterns.rn
n is a String
n is a vector
n is unknown
== () (1.0341ms)
```

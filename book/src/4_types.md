# Types

Types in Rune are identified uniquely by their *path*.
A path is a scope-separated identifier, like `std::float`.
This identifies a type object.

These can be used to perform basic type checking, like this:

```rune
use std::test::assert;

fn main() {
    assert(() is unit, "units should be units");
    assert(true is bool, "bools should be bools");
    assert('a' is char, "chars should be chars");
    assert(42 is int, "integers should be integers");
    assert(42.1 is float, "floats should be floats");
    assert("hello" is String, "strings should be strings");
    assert(#{"hello": "world"} is Object, "objects should be objects");
    assert(["hello", "world"] is Vec, "vectors should be vectors");
}
```

Conversely, the type check would fail if it's not valid:

```text
error: virtual machine error
  ┌─ .\scripts\book\4_2_bad_type_check.rn:4:5
  │
4 │     assert(["hello", "world"] is String, "vectors should be strings");
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  │     │
  │     virtual machine error
  │     assertion failed: vectors should be strings
  │     error in user-defined function
```

So this allows us to determine which type is which and act accordingly:

```rune
fn dynamic_type(n) {
    if n is String {
        dbg("n is a String");
    } else if n is Vec {
        dbg("n is a vector");
    } else {
        dbg("n is unknown");
    }
}

fn main() {
    dynamic_type("Hello");
    dynamic_type([1, 2, 3, 4]);
    dynamic_type(42);
}
```

```text
0 = String("n is a string")
0 = String("n is a vector")
0 = String("I don\'t know n")
```

A tighter way to accomplish this would be with a type switch:

```rune
fn dynamic_type(n) {
    switch n {
        n if n is String => dbg("n is a String"),
        n if n is Vec => dbg("n is an Vec"),
        _ => dbg("n is unknown"),
    }
}
```
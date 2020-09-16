# Structs

Structs are like objects, except that they have a predefined structure with a
set of keys that are known at compile time and guaranteed to be defined.

Structs can also, like most types, have an `impl` block associated with them
which creates instance functions that you can call on an instance of that
struct.

```rune
{{#include ../../scripts/book/structs/user_database.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/structs/user_database.rn
setbac is inactive
setbac is active
== () (6.2095ms)
```

Structs can also be pattern matched, like most types.

But since the fields of a struct are known at compile time, the compiler can
ensure that you're only using fields which are defined.

```rune
{{#include ../../scripts/book/structs/struct_matching.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/structs/struct_matching.rn
Yep, it's setbac.
Other user: newt.
== () (1.0652ms)
```
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
$> cargo run -- run scripts/book/structs/user_database.rn
setbac is inactive.
setbac is active.
```

An `impl` block can also declare constants, which belong to the type the same
way its functions do. They are reached through the type's name, or through
`Self` inside the `impl`, and can be used anywhere a constant can, including in
the definition of another one. Like any `const` they are evaluated when the
script is compiled, and they may be made `pub`.

```rune
{{#include ../../scripts/book/structs/associated_constants.rn}}
```

```text
$> cargo run -- run scripts/book/structs/associated_constants.rn
max: 255, grey: 127
```

A constant cannot yet hold an instance of a struct or enum defined in a script,
so `const BLACK = Self { r: 0, g: 0, b: 0 }` is a compile error.

Structs can also be pattern matched, like most types.

But since the fields of a struct are known at compile time, the compiler can
ensure that you're only using fields which are defined.

```rune
{{#include ../../scripts/book/structs/struct_matching.rn}}
```

```text
$> cargo run -- run scripts/book/structs/struct_matching.rn
Yep, it's setbac.
Other user: newt.
```

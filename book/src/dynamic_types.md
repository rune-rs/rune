# Dynamic types

Dynamic types are types which can be defined and used solely within a Rune
script. They provide the ability to structure data and associate functions with
it.

The following is a quick example of a `struct`:

```rune
{{#include ../../scripts/book/dynamic_types/greeting.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/dynamic_types/greeting.rn
Greetings from John-John Tedro, and good luck with this section!
== () (2.7585ms)
```

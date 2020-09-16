# Pattern Matching

In this section we will be discussing *Pattern Matching*.

Pattern matching is a flexible mechanism that allows for validating the
structure and type of the argument, while also destructuring it to give easy
access to what you need.

Below are some examples of its common uses to match on branch conditions:

```rune
{{#include ../../scripts/book/pattern_matching/big_match.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/pattern_matching/big_match.rn
The number one.
Another number: 2.
A vector starting with one and two, followed by 42.
One, but this time as a string.
Something else. Can I go eat now?
== () (5.691ms)
```

We will be covering each of these variants in detail in the coming sections.

## Matching Literals

Literals are the simplest form of matching, where we test if the branch is
exactly equal to a literal.

Literals take a number of form:

* A literal unit, simply `()`.
* A literal boolean, like `true` or `false`.
* A literal character, like `'a'` or `'あ'`.
* A literal integer, like `42`.
* A string, like `"Steven Universe"`.
* A vector, like the numbers `[4, 8, 15, 16, 23, 42]` or the empty vector `[]`.
* A tuple, like `("Steven Universe", 42)`.
* An object, like the numbers `{"name": "Steven Universe"}` or the empty `{}`.

Finally, literals can be *any* combination of the above.
Even `{"items": ["Sword", "Bow", "Axe"]}` is a literal that can be matched over.

## Match Bindings

In a pattern, every literal value can also be replaced with an ignore directive
or a binding.

The ignore directive looks like an underscore `_`, which tells rune to *ignore*
the value, allowing it to have any value.

```rune
{{#include ../../scripts/book/pattern_matching/ignore.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/pattern_matching/ignore.rn
Second item in vector is 2.
== () (281.3µs)
```

In contrast to ignoring, we can also *bind* the value to a variable that is then
in scope of the match arm.

```rune
{{#include ../../scripts/book/pattern_matching/bind.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/pattern_matching/bind.rn
Second item in vector is 2.
== () (6.25ms)
```

Here are some more examples:

* `[_, a, b]` which will ignore the first, but then capture the second and third
  element in the vector.
* `{"name": name}` will capture the `name` value out of the specified object.

Finally we can also add the sequence `..` to ask Rune to *ignore* any additional
values in a collection that might be present when matching a vector or an
object.

```rune
{{#include ../../scripts/book/pattern_matching/fast_cars.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/pattern_matching/fast_cars.rn
Pretty fast!
Can't tell 😞
What, where did you get that?
== () (5.3533ms)
```

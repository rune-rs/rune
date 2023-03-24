# Pattern matching

In this section we will be discussing *Pattern Matching*.

Pattern matching is a flexible mechanism that allows for validating the
structure and type of the argument, while also destructuring it to give easy
access to what you need.

Below are some examples of its common uses to match on branch conditions:

```rune
{{#include ../../scripts/book/pattern_matching/big_match.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/pattern_matching/big_match.rn
The number one.
Another number: 2.
A vector starting with one and two, followed by 42.
One, but this time as a string.
Something else. Can I go eat now?
```

We will be covering each of these variants in detail in the coming sections.

## Patterns

Things that can be matched over are called *patterns*, and there's a fairly
large number of them. In this section we'll try to document the most common
ones.

Patterns that can be matched over are the following:

* A unit, simply `()`.
* A boolean value, like `true` or `false`.
* A byte, like `b'a'` or `b'\x10'`.
* A character, like `'a'` or `'あ'`.
* An integer, like `42`.
* A string, like `"Steven Universe"`.
* A vector, like the numbers `[1, _, ..]`, or simply the empty vector `[]`. The
  values in the vectors are patterns themselves.
* A tuple, like `("Steven Universe", _, 42)`. The values in the tuple are
  patterns themselves.
* An object, like the numbers `{"name": "Steven Universe", "age": _}`, or the
  empty `{}`. The values in the object are patterns themselves.

Structs can be matched over by prefixing the match with their name:
* A unit struct: `Foo`.
* A tuple struct: `Foo(1, _)`.
* An object struct: `Foo { bar: 1, .. }`.

Similarly, variants in an enum can be matched over as well in the same way:
* A unit variant: `Foo::Variant`.
* A tuple variant: `Foo::Variant(1, _)`.
* An object variant: `Foo::Variant { bar: 1, .. }`.

Patterns can be almost *any* combination of the above. Even `{"items": ["Sword",
"Bow", "Axe"]}` is a pattern that can be matched over.

Anything that qualifies as a collection can have `..` as a suffix to match the
case that there are extra fields or values which are not covered in the pattern.
This is called a *rest pattern*.

```rune
{{#include ../../scripts/book/pattern_matching/rest_pattern.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/pattern_matching/rest_pattern.rn
```

## Binding and ignoring

In a pattern, every value can be replaced with a *binding* or an *ignore
pattern*. The ignore pattern is a single underscore `_`, which informs Rune that
it should ignore that value, causing it to match unconditionally regardless of
what it is.

```rune
{{#include ../../scripts/book/pattern_matching/ignore.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/pattern_matching/ignore.rn
Second item in vector is 2.
```

In contrast to ignoring, we can also *bind* the value to a variable that is then
in scope of the match arm. This will also match the value unconditionally, but
give us access to it in the match arm.

```rune
{{#include ../../scripts/book/pattern_matching/bind.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/pattern_matching/bind.rn
Second item in vector is 2.
```

Here are some more examples:

* `[_, a, b]` which will ignore the first value in the vector, but then bind the
  second and third as `a` and `b`.
* `{"name": name}` will bind the value `name` out of the specified object.

```rune
{{#include ../../scripts/book/pattern_matching/fast_cars.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/pattern_matching/fast_cars.rn
Pretty fast!
Can't tell 😞
What, where did you get that?
```

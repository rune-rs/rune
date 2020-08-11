# Pattern Matching

In this section we will be discussing *Pattern Matching*.

Pattern matching is a flexible mechanism that allows for validating the
structure and type of the argument, while also destructing it to give easy
access to what you need.

Below are some examples of its most common use to match on branch conditions:

```rust,noplaypen
fn main() {
    let x = 1;

    match x {
        1 => println!("the number one"),
        n if n is int => println!("n is a number"),
        [1, 2, ..] => println!("vector starting with one and two"),
        "one" => println!("one as a string"),
        _ => println!("anything"),
    }
}
```

We will be covering each of these variants in detail in the coming sections.

## Matching Literals

Literals are the simplest form of matching. Where we test if the branch is
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

```rust,noplaypen
fn test_ignore(vector) {
    match vector {
        [_, 42] => dbg("second item in vector is 42"),
    }
}
```

In contrast to ignoring, we cal also *bind* the value to a variable:

```rust,noplaypen
fn test_bind(vector) {
    match vector {
        [_, b] => dbg(`second item in vector is {b}`),
    }
}
```

Here are some more examples:

* `[_, a, b]` which will ignore the first, but then capture the second and third
  element in the vector.
* `{"name": name}` will capture the `name` value out of the specified object.

Finally we can also add the sequence `..` to ask Rune to *ignore* any additional
values in a collection that might be present when matching a vector or an
object.

```rust,noplaypen
/// Describe how fast the first car in the vector is.
fn first_car_speed(cars) {
    match cars {
        [first, ..] => match first {
            {"model": "Ford", "make": 2000, ..} => "Pretty fast",
            {"model": "Ford", "make": 1908, ..} => "Could be faster",
            _ => "Unknown",
        },
        _ => "You didn't give me a vector of cars!",
    }
}
```
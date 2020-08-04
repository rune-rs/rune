# Pattern Matching

In this section we will be discussing *Pattern Matching*.
In other languages this might also be known as *unpacking* or *destructuring*.

Pattern matching is a flexible mechanism that allows for validating the
structure and type of the argument, while also assigning it to useful bindings.

Below are some examples of its most common use to match on branch conditions:

```rust,noplaypen
fn main() {
    let x = 1;

    match x {
        1 => println!("the number one"),
        n if n is int => println!("n is a number"),
        [1, 2, ..] => println!("array starting with one and two"),
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

* A literal unit, simply `()`. This is the default "value does not exist" value
  for Rune.
* A literal boolean, like `true`.
* A literal character, like `'a'` or `'あ'`.
* A literal integer, like `42`.
* A string, like `"Steven Universe"`.
* An array, like the numbers `[4, 8, 15, 16, 23, 42]` or the empty `[]`.
* An object, like the numbers `{"name": "Steven Universe"}` or the empty `{}`.

Finally, literals can be *any* combination of the above.
Even `{"items": ["Sword", "Bow", "Axe"]}` is a literal that can be matched over.

## Match Bindings

In terms of pattern matching, each value can also be replaced with an `_`, which
tells Rune to *ignore* the value. Or a variable identifier like `name` which
tells rune to bind the value to that variable.

* `[_, a, b]` which will ignore the first, but then capture the second and third
  element in the array.
* `{"name": name}` will capture the `name` value out of the specified object.

The sequence `..` asks Rune to *ignore* any additional values that might be
present when matching an array or an object.

```rust,noplaypen
/// Describe how fast the first car in the array is.
fn first_car_speed(cars) {
    match cars {
        [first, ..] => match first {
            {"model": "Ford", "make": 2000, ..} => "Pretty fast",
            {"model": "Ford", "make": 1908, ..} => "Could be faster",
            _ => "Unknown",
        },
        _ => "You didn't give me an array of cars!",
    }
}
```
# Pattern Matching

In this section we will be discussing *Pattern Matching*.
In other languages this might also be known as *unpacking* or *destructuring*.

Pattern matching is a flexible mechanism that allows for validating the
structure and type of the argument, while also assigning it to useful bindings.

Below are some examples of its most common use to match on branch conditions:

```rune
let x = 1;

match x {
    1 => println!("the number one"),
    n if n is int => println!("n is a number"),
    [1, 2, ...] => println!("array starting with one and two"),
    "one" => println!("one as a string"),
    _ => println!("anything"),
}
```

We will be covering each of these variants in detail in the coming sections.

## Matching Literals

This is the simplest form of matching. Where we test if the branch is exactly
equal to a literal.
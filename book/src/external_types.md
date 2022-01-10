# External types

When a type is declared outside of Rune it is said to be *external*. External
types are declared when setting up native modules. And Rune allows various
levels of integration with the language.

On the simplest level an external type is entirely opaque. Rune knows nothing
about it except that it is a value bound to a variable.

Below is the most simple example of an external type. It's implemented by
deriving [Any] which can do a lot of heavy lifting for us.

```rust,noplaypen
{{#include ../../examples/examples/simple_external.rs}}
```

This type isn't particularly useful. Attempting to access a field on `external`
would simply error. We have to instruct Rune how the field is accessed.

Luckily [Any] allows us to easily do that by marking the fields we want to make
accessible to Rune with `#[rune(get)]`.

```rust,noplaypen
#[derive(Debug, Any)]
struct External {
    #[rune(get)]
    value: u32,
}
```

With our newfound power we can now read `external.value`.

```rune
pub fn main(external) {
    println!("{}", external.value);
}
```

Setting the value is similarly simple. We simply mark the field with
`#[rune(set)]`.

```rust,noplaypen
#[derive(Debug, Any)]
struct External {
    #[rune(get, set)]
    value: u32,
}
```

And now we can both read and write to `external.value`.

```rune
pub fn main(external) {
    external.value = external.value + 1;
}
```

> Note: See the section about [Field Functions](./field_functions.md) for a
> complete reference of the available attributes.

# External enums

Enums have a few more tricks that we need to cover. We want to be able to
*pattern match* and *construct* external enums.

There are three kinds of variants in an enum:
* Unit variants which have no fields. E.g. `External::Unit`.
* Tuple variants which have *numerical* fields. E.g. `External::Tuple(1, 2, 3)`.
* Struct variants which have *named* fields. E.g. `External::Struct { a: 1, b:
  2, c: 3 }`.

Pattern matching is supported out of the box. The only thing to take note of is
that pattern matching will only see fields that are annotated with
`#[rune(get)]`.

So the following type:

```rust,noplaypen
enum External {
    First(#[rune(get)] u32, u32),
    Second(#[rune(get)] u32),
}
```

Could be pattern matched like this in Rune:

```rune
pub fn main(external) {
    match external {
        External::First(a) => a,
        External::Second(b) => b,
    }
}
```

Take note on how `External::First` only "sees" the field marked with
`#[rune(get)]`.

Let's add a struct variant and see what we can do then:

```rust,noplaypen
enum External {
    First(#[rune(get)] u32, u32),
    Second(#[rune(get)] u32),
    Third {
        a: u32,
        b: u32,
        #[rune(get)]
        c: u32,
    },
}
```

And let's add `Third` to our example:

```rune
pub fn main(external) {
    match external {
        External::First(a) => a,
        External::Second(b) => b,
        External::Third { c } => b,
    }
}
```

## Constructing enum variants

Unit and tuple variants can be annotated with `#[rune(constructor)]` which is
necessary to allow for building enums in Rune. But in order for the constructor
to work, all fields **must** be annotated with `#[rune(get)]`.

```rust,noplaypen
enum External {
    #[rune(constructor)]
    First(#[rune(get)] u32, #[rune(get)] u32),
    #[rune(constructor)]
    Second(#[rune(get)] u32),
    Third {
        a: u32,
        b: u32,
        #[rune(get)]
        c: u32,
    },
}
```

```rune
pub fn main() {
    External::First(1, 2)
}
```

But why do we have the `#[rune(get)]` requirement? Consider what would happen
otherwise. How would we construct an instance of `External::First` without being
able to *specify* what the values of all fields are? The answer is that all
fields must be visible. Alternatively we can declare another constructor as an
associated function. The same way we'd do it in Rust.

[Any]: https://docs.rs/rune/latest/rune/derive.Any.html

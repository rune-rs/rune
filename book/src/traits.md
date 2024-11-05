# Traits

Traits in rune defines a collection associated items. Once a trait is
implemented by a type we can be sure that all the associated names it defines
are present on the type.

Traits allow us to reason about types more abstractly, such as this is an
iterator.

#### Limits

As usual, Rune doesn't permit more than one definition of an associated name.
Attempting to define more than one with the same name results in a build-time
error. This is in contrast to Rust which allows multiple traits with overlapping
methods to be defined. So why doesn't Rune allow for this?

Since Rune is a dynamic language, consider what would happen in a situation like
this:

```rust
struct Foo {
    /* .. */
}

impl Iterator for Foo {
    fn next(self) {
        /* .. */
    }
}

impl OtherIterator for Foo {
    fn next(self) {
        /* .. */
    }
}

let foo = Foo {
    /* .. */
};

// Which implementation of `next` should we call?
while let Some(value) = foo.next() {

}
```

Since there are no type parameters we can't solve the ambiguity by either only
having one trait defining the method in scope or by using an unambigious
function qualified function call.

#### Implementation

In the background the user-facing implementation of traits is done by
implementing protocols just before. Protocols are still used by the virtual
machine to call functions.

The separation that protocols provide is important because we don't want a user
to *accidentally* implement an associated method which would then be picked up
by a trait. Protocols are uniquely defined in their own namespace and cannot be
invoked in user code.

As an example, to implement the `Iterator` trait you have to implement the
`NEXT` protocol. So if the `NEXT` protocol is present and we request that the
`::std::iter::Iterator` trait should be implemented, the `NEXT` protocol
implementation is used to construct all the relevant associated methods. This is
done by calling `Module::implement_trait`.

```rust
let mut m = Module::with_item(["module"]);
m.ty::<Iter>()?;
m.function_meta(Iter::next__meta)?;
m.function_meta(Iter::size_hint__meta)?;
m.implement_trait::<Iter>(rune::item!(::std::iter::Iterator))?;

#[derive(Any)]
#[rune(item = "module")]
struct Iter {
    /* .. */
}

impl Iter {
    #[rune::function(keep, protocol = NEXT)]
    fn size_hint(&self) -> Option<bool> {
        Some(true)
    }

    #[rune::function(keep, protocol = SIZE_HINT)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, None)
    }
}
```

Note that this allows the `Iter` type above to specialize its `SIZE_HINT`
implementation. If the `SIZE_HINT` protocol was not defined, a default
implementation would be provided by the trait.

As a result of implementing the `::std::iter::Iterator` trait, the `Iter` type
now *automatically* gets all the iterator-associated function added to it. So
not only can you call `Iter::next` to advance the iterator, but also make use of
combinators such as `filter`:

```rust
let it = /* construct Iter */;

for value in it.filter(|v| v != true) {
    dbg!(value);
}
```

#### Defining a trait

Defining a trait is currently a low-level module operation. It's done by
implementing a handler which will be called to populate the relevant methods
when the trait is implement. Such as this snippet for the `Iterator` trait:

```rust
let mut m = Module::with_crate("std", ["iter"]);

let mut t = m.define_trait(["Iterator"])?;

t.handler(|cx| {
    let next = cx.find(&Protocol::NEXT)?;

    let size_hint = cx.find_or_define(&Protocol::SIZE_HINT, |_: Value| (0usize, None::<usize>))?;

    /* more methods */
    Ok(())
})?;
```

Calling `find` requires that `NEXT` is implemented. We can also see that the
implementation for `SIZE_HINT` will fall back to a default implementation if
it's not implemented. The appropriate protocol is also populated if it's
missing. All the relevant associated functions are also provided, such as
`value.next()` and `value.size_hint()`.

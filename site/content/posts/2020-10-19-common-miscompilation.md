+++
title = "Fixing a common miscompilation in Rune"
date = 2020-10-19
draft = false
template = "post.html"

[taxonomies]
categories = ["TMIR"]
tags = ["updates"]

[extra]
author = "John-John Tedro"
+++

Hopefully it should be no secret that Rune is a young project. And some
shortcuts have been taken when putting together the compiler. One such was how
items and their associated metadata was registered.

This particular shortcut happened to be subject to a common source of bugs which
desperately needed to be fixed. So in this post I'll describe the issue in the
hopes that it will be useful to other prospective language authors, and describe
how it was fixed.

<!-- more -->

Relevant pull requests: [#118](https://github.com/rune-rs/rune/pull/118), [#127](https://github.com/rune-rs/rune/pull/127).

Feel free to [**Discuss this on Reddit**](https://www.reddit.com/r/rust/comments/jdvc8r/this_month_and_a_half_in_rune/).

Each item in the language has a unique *name*. This is identified with a
scope-separated string, like `main::$0::bar`, which could be an inner function
`foo` inside of another function `bar`. The `$0` simply indicates that it
resides within the first anonymous scope inside of `main`.

> Because this item contains an anonymous component `$0` we say that it's
> *publicly unaddressable*. Unaddressable items is not usable outside of the
> scope in which it's defined. And we don't provide any public mechanisms for
> constructing the name easily. So they are used to "hide" things.

So the compiler broadly speaking is split up into two distinct steps.

* **Indexing**, during which the entire AST is walked over and language items
  are added to the query system.
* **Assembling**, at which point we process the AST to spit out instructions for
  the virtual machine. Things discovered during indexing are used to make
  assembly decisions.

A good example of why indexing has to be a distinct step. Consider how we
compile a generator. A function is a generator if it contains the `yield`
keyword. But we can't know if it does until we've walked through its content.
This is not something that can be determined in a single pass. We have to go
looking for a `yield` statement before we can decide how the function should be
assembled. From this requirement the indexing stage was born.

{% rune(footnote = "The main function calling a generator") %}
fn foo() {
    yield 42;
}

pub fn main() {
    foo()
}
{% end %}

During indexing we store whether a function is a generator or not in its
*metadata*. And this metadata used to be keyed by its item. So simplified the
above program resulted in the following compiler meta:

```text
foo: { type: function, is_generator: true }
main: { type: function, is_generator: false }
```

So far so good. Let's have a look at an example where we instead have a program
with two closures. Closures are compiled as special *unaddressable* functions:

{% rune(footnote = "A tuple with two closures") %}
pub fn main() {
    let a = || 1;
    let b = || 2;
    (a, b)
}
{% end %}

This program might contain the following items:

```text
main: the `main` function
main::$0::$0: first closure
main::$0::$1: second closure
```

Any *metadata* produced during indexing has to be looked up during assembly. To
access this during assembly the item was reconstructed. And in order to do this
faithfully we have to traverse the AST in the same order as during indexing. We
started noticing problems when we encountered code like this:

{% rune(footnote = "A tuple with two anonymous functions") %}
pub fn main() {
    (|| 1, || 2)
}
{% end %}

We have two options here. Either we assign the anonymous component `$0` to the
*first* or the *second* closure. This might seem trivially solvable by having a
rule like "always assign anonymous components from left to right", which is
essentially what we did. But not all operations are evaluated from left to
right. Take this for example:

{% rune(footnote = "A tuple with two anonymous functions") %}
pub fn main() {
    let object = #{};
    (|| object)()[(|| "key")()] = (|| "value")();
    object
}
{% end %}

> If you have a hard time visualizing what this evaluates to, it should simply
> result in `object["key"] = "value"`.

The evaluation order is the following, [which can try it for yourself in Rust](https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=a4544dd01d8a7840d68bca9515f9b4a9):

```text
second[third] = first;
```

As these sort of details were being fixed, the order in which the items were
constructed changed. It was too easy and sometimes complicated to fix the order
in which items were constructed during both phases. Miscompilations like this
could cause problems like the wrong closure being called if they happened to be
siblings in a complex expression.

Obviously this had to be fixed. So how did we do it?

#### Opaque identifiers in the AST

Instead of reconstructing the same item during assembly to lookup metadata
through, we assign the metadata once during indexing and construct an *opaque
identifier*. This is [assigned to the relevant AST] and is exclusively used to
lookup metadata. The query system [guarantees that each assigned identifier is
unique], which helps to avoid any conflicts.

This way it doesn't matter which order elements are being processed during
assembly. The identifier from the AST is simply used to safely look up the
correct compile time metadata. We don't need to invent and enforce a soft rule
like "always assign anonymous components from left to right". Something that
would otherwise require near-perfect test coverage to be effective.

[assigned to the relevant AST]: https://github.com/rune-rs/rune/blob/main/crates/rune/src/ast/path.rs#L14
[guarantees that each assigned identifier is unique]: https://github.com/rune-rs/rune/blob/main/crates/runestick/src/id.rs#L38

# Macros

Rune has (experimental) support for macros. These are functions which expand
into code, and can be used by library writers to "extend the compiler".

For now, the following type of macros are support:
* Function-like macros expanding to items (functions, type declarations, ..).
* Function-like macros expanding to expression (statements, blocks, async blocks, ..).

Macros can currently only be defined natively. This is to get around the rather
tricky issue that the code of a macro has to be runnable during compilation.
Native modules have an edge here, because they have to be defined at a time when
they are definitely available to the compiler.

> Don't worry though, we will be playing around with `macro fn` as well, but at
> a later stage 😉 (See [issue #27]).

Native modules also means we can re-use all the existing compiler infrastructure
for Rune as a library for macro authors. Which is really nice!

[issue #27]: https://github.com/rune-rs/rune/issues/27

## Writing a native macro

The following is the definition of the `stringy_math!` macro. Which is a macro
that can be invoked on expressions.

This relies heavily on a Rune-specific [`quote!` macro]. Which is inspired by its
[famed counterpart in the Rust world]. A major difference with Rune `quote!` is
that we need to pass in the `MacroContext` when invoking it. This is a detail
which will be covered in one of the advanced sections.

```rust,noplaypen
{{#include ../../crates/rune-macros/src/stringy_math_macro.rs}}
```

A macro is added to a [`Module`] using the [`Module::macro_`] function.

```rust,noplaypen
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "experiments"]);
    module.macro_(&["stringy_math"], stringy_math_macro::stringy_math)?;
    Ok(module)
}
```

With this module installed, we can now take `stringy_math!` for a spin.

```rune
{{#include ../../scripts/book/macros/stringy_math.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/macros/stringy_math.rn -O macros=true --experimental
200
== () (2.9737ms)
```

Until macros are considered somewhat stable, they will be hidden behind the `-O
macros=true` compiler option. This can be set programmatically in
[`Options::macros`]. `--experimental` is an option to Rune CLI which adds the
`std::experimental` module, which contains weird and experimental things like
`stringy_math!`.

[`quote!` macro]: https://docs.rs/rune/0/rune/macro.quote.html
[famed counterpart in the Rust world]: https://docs.rs/quote/1/quote/
[`Module`]: https://docs.rs/runestick/0/runestick/module/struct.Module.html
[`Module::macro_`]: https://docs.rs/runestick/0/runestick/module/struct.Module.html#method.macro_
[`Options::macros`]: https://docs.rs/rune/0/rune/struct.Options.html#method.macros
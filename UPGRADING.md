# Upgrading from 0.9.x to 0.10.x

## Crate merge

The `runestick` crate has been merged into `rune`. This means that anything that
previously imported items from `runestick` now has to be changed to import them
directly from `rune` instead.

In the process of doing this, we've also overhauled the public API of `rune`. So
some things are either no longer public because they don't need to be, or
they've been tucked into an appropriate submodule.

So something like this:

```rust
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, EmitDiagnostics, Options, Sources};
use runestick::{FromValue, Source, Vm};
use std::sync::Arc;
```

Becomes:

```rust
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Diagnostics, FromValue, Options, Sources, Source, Vm};
use std::sync::Arc;
```

Some types have been removed from the toplevel scope and might be found in one
of the new nested modules:

* [rune::compile] - For traits and types related to compiling. Like compiler
  metadata.
* [rune::macros] - For dealing with macros.
* [rune::parse] - For traits and types related to parsing.
* [rune::runtime] - For runtime-related things. This is where you'll find most
  of the things that were previously exported in [runestick].

[rune::compile]: https://docs.rs/rune/0.10.0/rune/compile/index.html
[rune::macros]: https://docs.rs/rune/0.10.0/rune/macros/index.html
[rune::parse]: https://docs.rs/rune/0.10.0/rune/parse/index.html
[rune::runtime]: https://docs.rs/rune/0.10.0/rune/runtime/index.html
[runestick]: https://docs.rs/runestick/0.9.1/runestick/

## Overhaul compile API

The compiler is now invoked by constructing a `rune::Build` instance through
`rune::prepare`. This allows for more easily using default values without having
to specify them during compilation (like [Options]). Consequently
[rune::load_sources], [rune::load_sources_with_visitor], and [rune::compile]
have all been removed.

[Options]: https://docs.rs/rune/0.9.1/rune/struct.Options.html
[rune::load_sources]: https://docs.rs/rune/0.9.1/rune/fn.load_sources.html
[rune::load_sources_with_visitor]: https://docs.rs/rune/0.9.1/rune/fn.load_sources_with_visitor.html
[rune::compile]: https://docs.rs/rune/0.9.1/rune/fn.compile.html

Replacing `rune::load_sources`:

```rust
let result = rune::load_sources(&context, &options, &mut sources, &mut diagnostics);
```

With:

```rust
let result = rune::prepare(&mut sources)
    .with_context(&context)
    .with_options(&options)
    .with_diagnostics(&mut diagnostics)
    .build();
```

Replacing `rune::load_sources_with_visitor`:

```rust
let result = rune::load_sources_with_visitor(
    &context,
    &options,
    &mut sources,
    &mut diagnostics,
    visitor.clone(),
    source_loader.clone(),
);
```

With:

```rust
let result = rune::prepare(&mut sources)
    .with_context(&context)
    .with_diagnostics(&mut diagnostics)
    .with_options(&options)
    .with_visitor(&mut visitor)
    .with_source_loader(&mut source_loader)
    .build();
```

Note that `rune::compile` does not offer a replacement since it uses the now
private `UnitBuilder` type.

## Overhaul the `emit` feature like `EmitDiagnostics`, `EmitSource`

The following traits have been removed in favor of being functions on the
appropriate type instead:

* [EmitDiagnostics] which is now [Diagnostics::emit] and [VmError::emit].
* [EmitSource] which is now [Source::emit_source_line].

The feature flag has also been renamed from `diagnostics` to `emit`.

[Diagnostics::emit]: https://docs.rs/rune/0.10.0/rune/struct.Diagnostics.html#method.emit
[EmitDiagnostics]: https://docs.rs/rune/0.9.1/rune/trait.EmitDiagnostics.html
[EmitSource]: https://docs.rs/rune/0.9.1/rune/trait.EmitSource.html
[Source::emit_source_line]: https://docs.rs/rune/0.10.0/rune/struct.Source.html#method.emit_source_line
[VmError::emit]: https://docs.rs/rune/0.10.0/rune/runtime/struct.VmError.html#method.emit

## Changed Macro API

Macros now take an explicit `ctx` argument in the form of [`&mut
MacroContext<'_>`][MacroContext].

This also requires the context to be "passed around" in certain places where it
didn't use to be necessary.

* [Lit::new] has been replaced with [MacroContext::lit].
* [Label::new] has been replaced with [MacroContext::label].
* [Ident::new] has been replaced with [MacroContext::ident].
* [rune::macros::eval] becomes [MacroContext::eval].
* [rune::macros::resolve] becomes [MacroContext::resolve].
* [rune::macros::stringify] becomes [MacroContext::stringify].

This is used instead of relying on TLS, which means that macros that used to be
written like this:

```rust
pub(crate) fn stringy_math(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match macros::resolve(op)?.as_ref() {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => return Err(SpannedError::msg(op.span(), "unsupported operation").into()),
        }
    }

    parser.eof()?;
    Ok(output.into_token_stream())
}
```

Must now instead do this:

```rust
pub(crate) fn stringy_math(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> rune::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, ctx.stream_span());

    let mut output = quote!(0);

    while !parser.is_eof()? {
        let op = parser.parse::<ast::Ident>()?;
        let arg = parser.parse::<ast::Expr>()?;

        output = match ctx.resolve(op)?.as_ref() {
            "add" => quote!((#output) + #arg),
            "sub" => quote!((#output) - #arg),
            "div" => quote!((#output) / #arg),
            "mul" => quote!((#output) * #arg),
            _ => return Err(SpannedError::msg(op.span(), "unsupported operation").into()),
        }
    }

    parser.eof()?;
    Ok(output.into_token_stream(ctx))
}
```

[Lit::new]: https://docs.rs/rune/0.9.1/rune/ast/enum.Lit.html#method.new
[Label::new]: https://docs.rs/rune/0.9.1/rune/ast/struct.Label.html#method.new
[Ident::new]: https://docs.rs/rune/0.9.1/rune/ast/struct.Ident.html#method.new
[MacroContext::eval]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.eval
[MacroContext::lit]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.lit
[MacroContext::label]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.label
[MacroContext::ident]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.ident
[MacroContext::resolve]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.resolve
[MacroContext::stringify]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html#method.stringify
[MacroContext]: https://docs.rs/rune/0.10.0/rune/macros/struct.MacroContext.html
[rune::macros::eval]: https://docs.rs/rune/0.9.1/rune/macros/fn.eval.html
[rune::macros::resolve]: https://docs.rs/rune/0.9.1/rune/macros/fn.resolve.html
[rune::macros::stringify]: https://docs.rs/rune/0.9.1/rune/macros/fn.stringify.html
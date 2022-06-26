# rune

<div align="center">
    <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
</div>

<br>

<div align="center">
<a href="https://rune-rs.github.io">
    <b>Visit the site 🌐</b>
</a>
-
<a href="https://rune-rs.github.io/book/">
    <b>Read the book 📖</b>
</a>
</div>

<br>

<div align="center">
<a href="https://github.com/rune-rs/rune/actions">
    <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/CI/badge.svg">
</a>

<a href="https://github.com/rune-rs/rune/actions">
    <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
</a>

<a href="https://crates.io/crates/rune">
    <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
</a>

<a href="https://docs.rs/rune">
    <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
</a>

<a href="https://discord.gg/v5AeNkT">
    <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
</a>
</div>

<br>

An embeddable dynamic programming language for Rust.

### Contributing

If you want to help out, there should be a number of optimization tasks
available in [Future Optimizations][future-optimizations]. Or have a look at
[Open Issues].

Create an issue about the optimization you want to work on and communicate that
you are working on it.

<br>

### Highlights of Rune

* Clean [Rust integration 💻][support-rust-integration].
* Memory safe through [reference counting 📖][support-reference-counted].
* [Template literals 📖][support-templates].
* [Try operators 📖][support-try].
* [Pattern matching 📖][support-patterns].
* [Structs and enums 📖][support-structs] with associated data and functions.
* Dynamic [vectors 📖][support-dynamic-vectors], [objects 📖][support-anon-objects], and [tuples 📖][support-anon-tuples] with built-in [serde support 💻][support-serde].
* First-class [async support 📖][support-async].
* [Generators 📖][support-generators].
* Dynamic [instance functions 📖][support-instance-functions].
* [Stack isolation 📖][support-stack-isolation] between function calls.
* Stack-based C FFI, like Lua's (TBD).

<br>

### Rune scripts

You can run Rune programs with the bundled CLI:

```
cargo run --bin rune -- run scripts/hello_world.rn
```

If you want to see detailed diagnostics of your program while it's running,
you can use:

```
cargo run --bin rune -- run scripts/hello_world.rn --dump-unit --trace --dump-vm
```

See `--help` for more information.

### Running scripts from Rust

> You can find more examples [in the `examples` folder].

The following is a complete example, including rich diagnostics using
[`termcolor`]. It can be made much simpler if this is not needed.

[`termcolor`]: https://docs.rs/termcolor

```rust
use rune::{Context, Diagnostics, FromValue, Source, Sources, Vm};
use rune::termcolor::{ColorChoice, StandardStream};
use std::sync::Arc;

#[tokio::main]
async fn main() -> rune::Result<()> {
    let context = Context::with_default_modules()?;
    let runtime = Arc::new(context.runtime());

    let mut sources = Sources::new();
    sources.insert(Source::new(
        "script",
        r#"
        pub fn add(a, b) {
            a + b
        }
        "#,
    ));

    let mut diagnostics = Diagnostics::new();

    let result = rune::prepare(&mut sources)
        .with_context(&context)
        .with_diagnostics(&mut diagnostics)
        .build();

    if !diagnostics.is_empty() {
        let mut writer = StandardStream::stderr(ColorChoice::Always);
        diagnostics.emit(&mut writer, &sources)?;
    }

    let unit = result?;
    let mut vm = Vm::new(runtime, Arc::new(unit));

    let output = vm.call(&["add"], (10i64, 20i64))?;
    let output = i64::from_value(output)?;

    println!("{}", output);
    Ok(())
}
```

[in the `examples` folder]: https://github.com/rune-rs/rune/tree/main/examples/examples
[future-optimizations]: https://github.com/rune-rs/rune/blob/main/FUTURE_OPTIMIZATIONS.md
[Open Issues]: https://github.com/rune-rs/rune/issues
[support-rust-integration]: https://github.com/rune-rs/rune/tree/main/crates/rune-modules
[support-reference-counted]: https://rune-rs.github.io/book/variables.html
[support-templates]: https://rune-rs.github.io/book/template_literals.html
[support-try]: https://rune-rs.github.io/book/try_operator.html
[support-patterns]: https://rune-rs.github.io/book/pattern_matching.html
[support-structs]: https://rune-rs.github.io/book/structs.html
[support-async]: https://rune-rs.github.io/book/async.html
[support-generators]: https://rune-rs.github.io/book/generators.html
[support-instance-functions]: https://rune-rs.github.io/book/instance_functions.html
[support-stack-isolation]: https://rune-rs.github.io/book/call_frames.html
[support-dynamic-vectors]: https://rune-rs.github.io/book/vectors.html
[support-anon-objects]: https://rune-rs.github.io/book/objects.html
[support-anon-tuples]: https://rune-rs.github.io/book/tuples.html
[support-serde]: https://github.com/rune-rs/rune/blob/main/crates/rune-modules/src/json.rs

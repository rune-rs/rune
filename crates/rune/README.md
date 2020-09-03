# rune

<div align="center">
<a href="https://rune-rs.github.io/rune/">
    <b>Read the Book 📖</b>
</a>
</div>

<br>

<div align="center">
<a href="https://github.com/rune-rs/rune/actions">
    <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
</a>

<a href="https://github.com/rune-rs/rune/actions">
    <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
</a>

<a href="https://discord.gg/v5AeNkT">
    <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
</a>
</div>

<br>

An embeddable dynamic programming language for Rust.

### Contributing

If you want to help out, there's a number of optimization tasks available in
[Future Optimizations][future-optimizations].

Create an issue about the optimization you want to work on and communicate that
you are working on it.

<br>

### Features of Rune

* [Clean Rust FFI][rust-ffi].
* Stack-based C FFI like with Lua (TBD).
* Stack frames, allowing for isolation across function calls.
* Template string ([Book 📖][support-templates]).
* Try operator ([Book 📖][support-try]).
* Pattern matching ([Book 📖][support-patterns]).
* Structs and enums with associated data and functions ([Book 📖][support-structs]).
* Asynchronous programming ([Book 📖][support-async]).
* Generators ([Book 📖][support-generators]).

<br>

### Rune Scripts

You can run Rune programs with the bundled CLI:

```
cargo run -- scripts/hello_world.rn
```

If you want to see detailed diagnostics of your program while it's running,
you can use:

```
cargo run -- scripts/hello_world.rn --dump-unit --trace --dump-vm
```

See `--help` for more information.

[rust-ffi]: https://github.com/rune-rs/rune/blob/master/crates/runestick-http/src/lib.rs
[future-optimizations]: https://github.com/rune-rs/rune/blob/master/FUTURE_OPTIMIZATIONS.md
[support-templates]: https://rune-rs.github.io/rune/4_6_template_strings.html
[support-try]: https://rune-rs.github.io/rune/6_try_operator.html
[support-patterns]: https://rune-rs.github.io/rune/4_5_pattern_matching.html
[support-structs]: https://rune-rs.github.io/rune/5_6_structs.html
[support-async]: https://rune-rs.github.io/rune/8_async.html
[support-generators]: https://rune-rs.github.io/rune/7_generators.html

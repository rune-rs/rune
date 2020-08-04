[![Build Status](https://github.com/udoprog/st/workflows/Build/badge.svg)](https://github.com/udoprog/st/actions)

# st

ST, a really simple stack-based virtual machine.

## Contributing

If you want to help out, there's a number of optimization tasks available in
[FUTURE_OPTIMIZATIONS.md].

Create an issue about the optimization you want to work on and communicate that
you are working on it.

## Features of st

* [Clean Rust FFI][rust-ffi].
* Stack-based C FFI like with Lua (TBD).
* Stack frames, allowing for isolation across function calls.
* A rust-like reference language called *Rune*.

## Rune Scripts

ST comes with a simple scripting language called *Rune*.

You can run example scripts through rune-cli:

```bash
cargo run -- ./scripts/controls.rn
```

If you want to see diagnostics of your unit, you can do:

```bash
cargo run -- ./scripts/controls.rn --dump-unit --trace
```

[rust-ffi]: https://github.com/udoprog/st/blob/master/crates/st-http/src/lib.rs
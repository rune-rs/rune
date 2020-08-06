[![Build Status](https://github.com/udoprog/stk/workflows/Build/badge.svg)](https://github.com/udoprog/stk/actions)

# stk

STK, a simple stack-based virtual machine.

### Contributing

If you want to help out, there's a number of optimization tasks available in
[Future Optimizations][future-optimizations].

Create an issue about the optimization you want to work on and communicate that
you are working on it.

### Features of stk

* [Clean Rust FFI][rust-ffi].
* Stack-based C FFI like with Lua (TBD).
* Stack frames, allowing for isolation across function calls.
* A rust-like reference language called *Rune*.

### Rune Scripts

stk comes with a simple scripting language called *Rune*.

You can run example scripts through rune-cli:

```bash
cargo run -- ./scripts/hello_world.rn
```

If you want to see diagnostics of your unit, you can do:

```bash
cargo run -- ./scripts/hello_world.rn --dump-unit --trace
```

[rust-ffi]: https://github.com/udoprog/stk/blob/master/crates/stk-http/src/lib.rs
[future-optimizations]: https://github.com/udoprog/stk/blob/master/FUTURE_OPTIMIZATIONS.md

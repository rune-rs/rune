[![Build Status](https://github.com/udoprog/st/workflows/Build/badge.svg)](https://github.com/udoprog/st/actions)

# st

ST, a really simple stack-based virtual machine.

## Features of st

* [Clean Rust FFI][rust-ffi].
* Stack-based C FFI like with Lua (TBD).
* Stack frames, allowing for isolation across function calls.
* A rust-like reference language called *Rune*.

## Rune Scripts

ST comes with a simple scripting language called *Rune*.

You can run example scripts through rune-cli:

```bash
cargo rune-cli ./scripts/controls.rn
```

If you want to see diagnostics of your unit, you can do:

```bash
cargo rune-cli ./scripts/controls.rn --dump-unit --trace
```

[rust-ffi]: https://github.com/udoprog/st/blob/master/crates/st-http/src/lib.rs
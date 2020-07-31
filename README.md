[![Build Status](https://github.com/udoprog/st/workflows/Build/badge.svg)](https://github.com/udoprog/st/actions)

# st

ST, a really simple stack-based virtual machine.

## Features of ST

* Clean Rust ffi.
* Stack-based C ffi as with Lua (TBD).
* Stack frames, allowing for isolation across function calls.
* A rust-like reference language called *Rune*.
* No variable slots, we only use the stack to store and manipulate variables.

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
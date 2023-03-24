# Rune C API

This crate is not intended to be published, but instead contains the necessary
bindings to provide a C API for Rune.

## Building and running examples

Examples are built using meson, and requires that the static library is already
built and available in `target/debug` (or `target/release`). Note that for
cbindgen to work it requires `+nightly`.

```
cargo build --package rune-capi
meson setup builddir capi
ninja -C builddir
```

After this, you can find the binaries corresponding to their names in
[`examples`](examples) in `target/builddir`.

When building and running on Windows you might have to run through the [MSVC
development
shell](https://docs.microsoft.com/en-us/visualstudio/ide/reference/command-prompt-powershell)
to have access to the C compiler.

## Regenerating header file

Since we use macros, the header can only be regenerated using `+nightly`.

```sh
cargo +nightly run --package rune-capi --bin cbindgen
```

# wasm

Builds the playground in `crates/rune-wasm` and writes the wasm-bindgen output
and the `rune.js` glue to `site/static/js`. Loading `rune.js` starts the editor
on every `.rune` element of the page:

```
cargo run -p wasm
```

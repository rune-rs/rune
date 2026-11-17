# site

Builds <https://rune-rs.github.io> from `site/`: the pages are yew components
rendered to static HTML, and the playground editor and theme toggle are hydrated
in the browser by the `rune-wasm` build which this also produces.

```
cargo run -p site -- build              # writes target/site
cargo run -p site -- serve -p 8740      # builds, then serves and re-renders on change
```

`--no-wasm` skips the wasm build and reuses what is already in the output.

# The Rune Programming Language Book

This is a book built with [mdbook](https://github.com/rust-lang/mdBook).

You can build the book with:

```bash
cargo install mdbook
mdbook build --open
```

## highlight.js fork

This book uses a [custom fork] of highlight.js with support for rune.

[custom fork]: https://github.com/rune-rs/highlight.js/tree/rune

The fork is built using:

```bash
npm install
node tools/build.js -h :common
```

Then you copy `build/highlight.js.min` to `src/highlight.js`.

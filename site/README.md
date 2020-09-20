## Rune Site

The static site for https://rune-rs.github.io/

Based on the [Ergo Theme](https://www.getzola.org/themes/ergo/).

#### Contributing

So you want to work on the site?

It has the following prerequisities:

* [Rust and Cargo](https://www.rust-lang.org/).
* [Node and NPM](https://nodejs.org).
* [Zola](https://www.getzola.org/) version 11 (12 has a bug which prevents posts
  from rendering).

The first thing you need to do is rebuild `rune-wasm`:

```
cd crates/rune-wasm
npm i
npm run build
```

Now you can run the Zola site:

```
cd site
zola serve
```
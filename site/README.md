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

#### Custom Ace Mode

We maintain a branch of the Ace Editor which has a special mode for rune. It can
be found in the [`rune` branch of `rune-rs/ace`](https://github.com/rune-rs/ace/tree/rune).

You can build the mode by doing the following in the ace repo:

```
$> npm i
$> node Makefile.dryice.js normal
$> cp .\build\src-min\mode-rune.js ..\rune\site\static\ace\
```

> Note: You'll need to adjust `..\rune\site\static\ace\` to point to your actual
> checkout of rune if that differs.

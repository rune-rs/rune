# Rune site

The site at <https://rune-rs.github.io/>, written in Rust: a yew application in
`crates/rune-wasm` over content which `tools/site` builds from this directory.

```
cargo run -p site -- serve        # http://127.0.0.1:8740/, re-rendered on change
cargo run -p site -- build        # target/site
```

- `site.toml` - the title, links and the playground's starting program.
- `content/index.md` - the front page, below its header.
- `content/<name>.md` - a page at `/<name>/`.
- `content/posts/<date>-<name>.md` - a post at `/posts/<name>/`. Anything else
  in `content/` is copied as is, so a post's images sit next to it.
- `static/` - copied to the root of the site, including `style.css`.

Pages and posts start with TOML front matter between `+++` lines:

```
+++
title = "Rune 0.13"
date = 2023-10-10
author = "John-John Tedro"
description = "Optional, for link previews."
+++
```

A post's summary on the blog page is everything above a `<!-- more -->` line,
or its first paragraph.

A ```` ```rune ```` block becomes a runnable editor. Attributes go after the
language as a TOML inline table:

````
```rune { caption = "Shown under the editor", manual = true }
pub fn main() {
    42
}
```
````

`manual` runs it only with its button rather than as it is edited, `options` is
a list of compiler options, and `config` is merged into the editor's compile
configuration (`config = { suppress_text_warnings = true }`). Lines starting
with `# ` are run but not shown. Other languages are highlighted statically.

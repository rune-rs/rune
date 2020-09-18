+++
title = "Hello Internet"
date = 2020-09-18
template = "post.html"

[taxonomies]
categories = []
tags = ["updates"]

[extra]
author = "John-John Tedro"
+++

Less then one month ago [I announced Rune on
Reddit](https://www.reddit.com/r/rust/comments/in67d3/introducing_rune_a_new_stackbased_dynamic/).
And the response has been amazing.

One of the issues raised were [issue
#45](https://github.com/rune-rs/rune/issues/45), **Community Site for Rune**.
This site is an attempt to address that.

<!-- more -->

So let's talk for a second about the tech behind this site. It's driven by
[Zola](https://www.getzola.org/) and is deployed automatically through [GitHub
Actions](https://github.com/rune-rs/rune/actions?query=workflow%3ASite) on every
push to master. [There's a little bit of glue
involved](https://github.com/rune-rs/rune/tree/master/tools/site) to
download and run Zola, but apart from that the experience has been really
smooth. It's a great project overall.

```rust
fn main() -> Result<()> {
    let url = match env::var("ZOLA_URL") {
        Ok(url) => url,
        Err(..) => bail!("missing ZOLA_URL"),
    };

    let target = Path::new("target");
    let bin = target.join("zola");

    if !bin.is_file() {
        println!("Downloading: {}", url);
        let bytes = reqwest::blocking::get(&url)?.bytes()?;
        let decoder = GzDecoder::new(io::Cursor::new(bytes.as_ref()));
        let mut archive = Archive::new(decoder);
        archive.unpack(target)?;
    }

    if !bin.is_file() {
        bail!("Missing bin: {}", bin.display());
    }

    let mut it = env::args();
    it.next();

    let status = Command::new(bin).args(it).status()?;
    std::process::exit(status.code().unwrap());
}
```

Basically it's perfect. It's part of the repository and I don't have to think
too much about it. Hopefully others feel the same and that the threshold for
contributing to the site is as minimal as possible.

So now that I've set up the skeleton for it. Let's build something cool!

P.S. And as a final treat, here's a code snippet that you can edit and run! 😊

{% rune(footnote = "Showcasing the integrated editor. Neat, huh?") %}
fn main() {
    println("Hello World!");
}
{% end %}
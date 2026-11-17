//! Builds <https://rune-rs.github.io>.

mod config;
mod content;
mod date;
mod feed;
mod highlight;
mod serve;
mod wasm;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const USAGE: &str = "usage: site <build|serve> [-o DIR] [-p PORT] [--bind ADDR] [--no-wasm]";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();

    let mut out = None;
    let mut port = 8740;
    let mut bind = String::from("127.0.0.1");
    let mut wasm = true;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(args.next().context("missing directory")?)),
            "-p" | "--port" => port = args.next().context("missing port")?.parse()?,
            "--bind" => bind = args.next().context("missing address")?,
            "--no-wasm" => wasm = false,
            other => bail!("unknown argument `{other}`\n{USAGE}"),
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let site = root.join("site");
    let out = out.unwrap_or_else(|| root.join("target/site"));

    match command.as_str() {
        "build" => {
            if wasm {
                wasm::build(&root, &out)?;
            }

            build(&site, &out)?;
        }
        "serve" => {
            if wasm {
                wasm::build(&root, &out)?;
            }

            build(&site, &out)?;
            serve::run(&site, &out, &bind, port)?;
        }
        _ => bail!("{USAGE}"),
    }

    Ok(())
}

/// Render the site into `out`, leaving the wasm build in `out/js` alone.
pub fn build(site: &Path, out: &Path) -> Result<()> {
    let content = content::load(site)?;

    if out.is_dir() {
        for entry in fs::read_dir(out)? {
            let entry = entry?;

            if entry.file_name() != "js" {
                let path = entry.path();

                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    write_json(&out.join("content/site.json"), &content.site)?;
    write_json(&out.join("content/home.json"), &content.home)?;

    for page in &content.pages {
        write_json(
            &out.join(format!("content/pages/{}.json", page.name)),
            &page.document,
        )?;
    }

    for post in &content.posts {
        write_json(
            &out.join(format!("content/posts/{}.json", post.name)),
            &post.document,
        )?;
    }

    // Every address the app answers is a file too, so that it can be linked to
    // and reloaded on a host which serves files and nothing else.
    let config = &content.config;
    write(
        &out.join("index.html"),
        shell(config, None, None).as_bytes(),
    )?;
    write(
        &out.join("404.html"),
        shell(config, Some("Not found"), None).as_bytes(),
    )?;
    write(
        &out.join("posts/index.html"),
        shell(config, Some("Blog"), None).as_bytes(),
    )?;
    write(
        &out.join("play/index.html"),
        shell(config, Some("Playground"), None).as_bytes(),
    )?;

    for entry in content.pages.iter() {
        let html = shell(config, Some(&entry.title), entry.description.as_deref());
        write(&out.join(&entry.name).join("index.html"), html.as_bytes())?;
    }

    for entry in content.posts.iter() {
        let html = shell(config, Some(&entry.title), entry.description.as_deref());
        write(
            &out.join("posts").join(&entry.name).join("index.html"),
            html.as_bytes(),
        )?;
    }

    write(&out.join("rss.xml"), feed::rss(&content).as_bytes())?;
    write(&out.join("sitemap.xml"), feed::sitemap(&content).as_bytes())?;

    copy_tree(&site.join("static"), out)?;

    for asset in &content.assets {
        let to = out.join(asset.strip_prefix(site.join("content"))?);
        write(&to, &fs::read(asset)?)?;
    }

    Ok(())
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    write(path, serde_json::to_string(value)?.as_bytes())
}

/// The document the app starts in, with what a link preview or a crawler
/// which does not run it should know about the page.
fn shell(config: &config::Config, title: Option<&str>, description: Option<&str>) -> String {
    use highlight::escape;

    let title = match title {
        Some(title) => format!("{} | {}", escape(title), escape(&config.title)),
        None => escape(&config.title),
    };

    let description = escape(description.unwrap_or(&config.description));

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<link rel="icon" href="/img/logo.png">
<link rel="stylesheet" href="/style.css">
<link rel="alternate" type="application/rss+xml" title="{site}" href="/rss.xml">
<script>try{{var t=localStorage.getItem("theme");if(t==="light"||t==="dark")document.documentElement.dataset.theme=t}}catch(e){{}}</script>
<script type="module" src="/js/rune.js"></script>
</head>
<body>
<div id="app"><div class="boot" aria-busy="true"></div></div>
<noscript><p class="noscript">This site is an application written in Rust, and needs JavaScript and WebAssembly to run.</p></noscript>
</body>
</html>
"#,
        site = escape(&config.title),
    )
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, bytes).with_context(|| path.display().to_string())
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let target = to.join(entry.file_name());

        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            write(&target, &fs::read(&path)?)?;
        }
    }

    Ok(())
}

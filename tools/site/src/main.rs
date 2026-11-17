//! Builds <https://rune-rs.github.io>.

mod config;
mod content;
mod date;
mod digest;
mod feed;
mod highlight;
mod home;
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

    if !matches!(command.as_str(), "build" | "serve") {
        bail!("{USAGE}");
    }

    let js = if wasm {
        wasm::build(&root, &out)?
    } else {
        wasm::current(&out)?
    };

    build(&site, &out, &js)?;

    if command == "serve" {
        serve::run(&site, &out, &js, &bind, port)?;
    }

    Ok(())
}

/// What other builds put in the output: the wasm here, and the book and the
/// generated documentation in the site workflow.
const KEEP: [&str; 4] = ["js", "book", "docs", "api"];

/// Where the pages find what they load, each named for what is in it so that
/// a page never meets a script, style or content from another build.
struct Assets<'a> {
    js: &'a str,
    style: String,
    content: String,
}

/// Render the site into `out`, leaving what other builds put there alone. `js`
/// is the directory under `out/js` which the wasm build wrote.
pub fn build(site: &Path, out: &Path, js: &str) -> Result<()> {
    let content = content::load(site)?;

    // A stale `site/static/js` from the old JavaScript build replaced the
    // loader the pages ask for, and the site never started.
    for reserved in KEEP.iter().chain(["content"].iter()) {
        let path = site.join("static").join(reserved);

        if path.exists() {
            bail!(
                "{} would overwrite what the build writes to `{reserved}`; remove it",
                path.display()
            );
        }
    }

    if out.is_dir() {
        for entry in fs::read_dir(out)? {
            let entry = entry?;

            if !KEEP.iter().any(|keep| entry.file_name() == *keep) {
                let path = entry.path();

                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    let mut documents = vec![
        (
            String::from("site.json"),
            serde_json::to_string(&content.site)?,
        ),
        (
            String::from("home.json"),
            serde_json::to_string(&content.home)?,
        ),
    ];

    for page in &content.pages {
        let json = serde_json::to_string(&page.document)?;
        documents.push((format!("pages/{}.json", page.name), json));
    }

    for post in &content.posts {
        let json = serde_json::to_string(&post.document)?;
        documents.push((format!("posts/{}.json", post.name), json));
    }

    let version = digest::of(
        documents
            .iter()
            .flat_map(|(path, json)| [path.as_bytes(), json.as_bytes()]),
    );

    // The unversioned copy is for a page loaded before the site was deployed
    // again, whose own version is gone.
    for (path, json) in &documents {
        write(
            &out.join("content").join(&version).join(path),
            json.as_bytes(),
        )?;
        write(&out.join("content").join(path), json.as_bytes())?;
    }

    let assets = Assets {
        js,
        style: digest::of([fs::read(site.join("static/style.css"))?.as_slice()]),
        content: format!("/content/{version}"),
    };

    // Every address the app answers is a file too, so that it can be linked to
    // and reloaded on a host which serves files and nothing else.
    let config = &content.config;
    let shell = |title, description| shell(config, &assets, title, description);

    write(&out.join("index.html"), shell(None, None).as_bytes())?;
    write(
        &out.join("404.html"),
        shell(Some("Not found"), None).as_bytes(),
    )?;
    write(
        &out.join("posts/index.html"),
        shell(Some("Blog"), None).as_bytes(),
    )?;
    write(
        &out.join("play/index.html"),
        shell(Some("Playground"), None).as_bytes(),
    )?;

    for entry in content.pages.iter() {
        let html = shell(Some(&entry.title), entry.description.as_deref());
        write(&out.join(&entry.name).join("index.html"), html.as_bytes())?;
    }

    for entry in content.posts.iter() {
        let html = shell(Some(&entry.title), entry.description.as_deref());
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

/// The document the app starts in, with what a link preview or a crawler
/// which does not run it should know about the page.
fn shell(
    config: &config::Config,
    assets: &Assets<'_>,
    title: Option<&str>,
    description: Option<&str>,
) -> String {
    use highlight::escape;

    let title = match title {
        Some(title) => format!("{} | {}", escape(title), escape(&config.title)),
        None => escape(&config.title),
    };

    let description = escape(description.unwrap_or(&config.description));

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48">
<link rel="icon" href="/img/icon-192.png" type="image/png" sizes="192x192">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0d1015" media="(prefers-color-scheme: dark)">
<link rel="stylesheet" href="/style.css?v={style}">
<link rel="alternate" type="application/rss+xml" title="{site}" href="/rss.xml">
<script>try{{var t=localStorage.getItem("theme");if(t==="light"||t==="dark")document.documentElement.dataset.theme=t}}catch(e){{}}</script>
<script type="module" src="/js/{js}/rune.js"></script>
</head>
<body>
<div id="app" data-content="{content}"><div class="boot" aria-busy="true"></div></div>
<noscript><p class="noscript">This site is an application written in Rust, and needs JavaScript and WebAssembly to run.</p></noscript>
</body>
</html>
"##,
        site = escape(&config.title),
        style = assets.style,
        js = assets.js,
        content = assets.content,
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

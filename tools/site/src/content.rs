//! Reads `site/` into the documents the app renders.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use rune_wasm::editor;
use rune_wasm::model::{Block, Document, PostSummary, Site};
use serde::Deserialize;

use crate::config::Config;
use crate::date::Date;
use crate::highlight::Highlighter;

/// Marks where a post's summary ends.
const MORE: &str = "<!-- more -->";
const WORDS_PER_MINUTE: usize = 200;

pub struct Content {
    pub config: Config,
    pub site: Site,
    pub home: Document,
    pub pages: Vec<Entry>,
    pub posts: Vec<Entry>,
    pub assets: Vec<PathBuf>,
}

pub struct Entry {
    /// The page's name, or the post's slug.
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub date: Option<Date>,
    pub document: Document,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Front {
    title: Option<String>,
    description: Option<String>,
    date: Option<toml::value::Datetime>,
    author: Option<String>,
}

/// The attributes of a ```` ```rune ```` block, written as a TOML inline table
/// after the language: ```` ```rune { caption = "...", manual = true } ````.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snippet {
    caption: Option<String>,
    /// Run with the button only, rather than as it is edited.
    #[serde(default)]
    manual: bool,
    #[serde(default)]
    options: Vec<String>,
    /// Merged into the editor's compile configuration.
    config: Option<toml::Table>,
}

pub fn load(site: &Path) -> Result<Content> {
    let config: Config = read_toml(&site.join("site.toml"))?;
    let content = site.join("content");
    let highlighter = Highlighter::new();

    let (_, body) = read_markdown(&content.join("index.md"))?;
    let home = Document {
        title: None,
        blocks: render(&highlighter, &body, true)?,
    };

    let mut pages = Vec::new();
    let mut posts = Vec::new();
    let mut summaries = Vec::new();
    let mut assets = Vec::new();

    for path in files(&content)? {
        let relative = path.strip_prefix(&content)?;

        if path.extension().is_none_or(|e| e != "md") {
            assets.push(path);
            continue;
        }

        if relative == Path::new("index.md") {
            continue;
        }

        let (front, body) = read_markdown(&path)?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .context("file name")?;

        let context = || path.display().to_string();
        let title = front
            .title
            .clone()
            .with_context(|| format!("{}: missing title", context()))?;

        match relative.parent().and_then(|p| p.to_str()) {
            Some("") => {
                pages.push(Entry {
                    name: stem.to_owned(),
                    title: title.clone(),
                    description: front.description.clone(),
                    date: None,
                    document: Document {
                        title: Some(title),
                        blocks: render(&highlighter, &body, true).with_context(context)?,
                    },
                });
            }
            Some("posts") => {
                let date = front
                    .date
                    .as_ref()
                    .with_context(|| format!("{}: missing date", context()))?;
                let date = Date::from_toml(date)?;
                let slug = slug(stem);

                let summary = match body.split_once(MORE) {
                    Some((summary, _)) => html(render(&highlighter, summary, false)?),
                    None => first_paragraph(&html(render(&highlighter, &body, false)?)),
                };

                let words = body.split_whitespace().count();

                summaries.push((
                    date,
                    PostSummary {
                        slug: slug.clone(),
                        title: title.clone(),
                        date: date.to_string(),
                        date_long: date.long(),
                        author: front.author.clone(),
                        minutes: words.div_ceil(WORDS_PER_MINUTE).max(1) as u32,
                        summary,
                    },
                ));

                posts.push(Entry {
                    name: slug,
                    title: title.clone(),
                    description: front.description.clone(),
                    date: Some(date),
                    document: Document {
                        title: Some(title),
                        blocks: render(&highlighter, &body, true).with_context(context)?,
                    },
                });
            }
            _ => bail!("{}: content is either a page or a post", context()),
        }
    }

    summaries.sort_by_key(|(date, _)| std::cmp::Reverse(*date));
    posts.sort_by_key(|post| std::cmp::Reverse(post.date));

    let site = Site {
        title: config.title.clone(),
        description: config.description.clone(),
        tagline: config.tagline.clone(),
        links: rune_wasm::model::Links {
            book: config.links.book.clone(),
            docs: config.links.docs.clone(),
            api: config.links.api.clone(),
            github: config.links.github.clone(),
            discord: config.links.discord.clone(),
        },
        playground: config.playground.clone(),
        pages: pages.iter().map(|p| p.name.clone()).collect(),
        posts: summaries.into_iter().map(|(_, s)| s).collect(),
    };

    Ok(Content {
        config,
        site,
        home,
        pages,
        posts,
        assets,
    })
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    toml::from_str(&text).with_context(|| path.display().to_string())
}

/// Front matter between `+++` lines, then markdown.
fn read_markdown(path: &Path) -> Result<(Front, String)> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;

    let Some(rest) = text.strip_prefix("+++\n") else {
        return Ok((Front::default(), text));
    };

    let Some((front, body)) = rest.split_once("\n+++\n") else {
        bail!("{}: unterminated front matter", path.display());
    };

    let front = toml::from_str(front).with_context(|| path.display().to_string())?;
    Ok((front, body.to_owned()))
}

fn files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut queue = vec![dir.to_owned()];

    while let Some(dir) = queue.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();

            if path.is_dir() {
                queue.push(path);
            } else {
                out.push(path);
            }
        }
    }

    out.sort();
    Ok(out)
}

/// `2023-10-10-rune-0.13.0` is served as `rune-0-13-0`.
fn slug(stem: &str) -> String {
    let bytes = stem.as_bytes();

    let dated = bytes.len() > 11
        && bytes[..10].iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
        && bytes[10] == b'-';

    let stem = if dated { &stem[11..] } else { stem };
    let mut out = String::new();

    for c in stem.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }

    out.trim_end_matches('-').to_owned()
}

fn html(blocks: Vec<Block>) -> String {
    let mut out = String::new();

    for block in blocks {
        if let Block::Html { html } = block {
            out.push_str(&html);
        }
    }

    out
}

fn first_paragraph(html: &str) -> String {
    match (html.find("<p>"), html.find("</p>")) {
        (Some(start), Some(end)) if start < end => html[start..end + 4].to_owned(),
        _ => String::new(),
    }
}

/// Markdown as blocks of HTML, split around the runnable snippets when
/// `editors` is set. Otherwise those are highlighted like any other code.
fn render(highlighter: &Highlighter, markdown: &str, editors: bool) -> Result<Vec<Block>> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;

    let mut blocks = Vec::new();
    let mut events = Vec::new();
    let mut code = None::<(String, String)>;
    let mut heading = None::<(HeadingLevel, Vec<Event>)>;
    let mut ids = HashSet::new();

    let flush = |events: &mut Vec<Event>, blocks: &mut Vec<Block>| {
        if !events.is_empty() {
            let mut html = String::new();
            pulldown_cmark::html::push_html(&mut html, events.drain(..));
            blocks.push(Block::Html { html });
        }
    };

    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };

                code = Some((info, String::new()));
            }
            Event::Text(text) if code.is_some() => {
                if let Some((_, body)) = &mut code {
                    body.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let Some((info, body)) = code.take() else {
                    continue;
                };

                let (lang, attributes) = split_info(&info);

                if lang == "rune" && editors {
                    let snippet = snippet(attributes)?;
                    let (source, prelude) = editor::Props::snippet(&body);

                    let config = match snippet.config {
                        Some(config) => Some(serde_json::to_string(&config)?),
                        None => None,
                    };

                    flush(&mut events, &mut blocks);

                    blocks.push(Block::Editor {
                        props: editor::Props {
                            source,
                            prelude,
                            update_url: false,
                            run_on_change: !snippet.manual,
                            options: snippet.options,
                            config,
                        },
                        caption: snippet.caption,
                    });
                } else {
                    let body = if lang == "rune" {
                        editor::Props::snippet(&body).0
                    } else {
                        body
                    };

                    events.push(Event::Html(CowStr::from(highlighter.block(lang, &body))));
                }
            }
            Event::Start(Tag::Heading { level, .. })
                if matches!(
                    level,
                    HeadingLevel::H2 | HeadingLevel::H3 | HeadingLevel::H4
                ) =>
            {
                heading = Some((level, Vec::new()));
            }
            Event::End(TagEnd::Heading(..)) if heading.is_some() => {
                let Some((level, inner)) = heading.take() else {
                    continue;
                };

                let mut text = String::new();

                for event in &inner {
                    if let Event::Text(t) | Event::Code(t) = event {
                        text.push_str(t);
                    }
                }

                let base = match slug(&text) {
                    slug if slug.is_empty() => String::from("section"),
                    slug => slug,
                };

                let mut id = base.clone();
                let mut n = 1;

                while !ids.insert(id.clone()) {
                    id = format!("{base}-{n}");
                    n += 1;
                }

                let tag = match level {
                    HeadingLevel::H2 => "h2",
                    HeadingLevel::H3 => "h3",
                    _ => "h4",
                };

                events.push(Event::Html(CowStr::from(format!("<{tag} id=\"{id}\">"))));
                events.extend(inner);
                events.push(Event::Html(CowStr::from(format!(
                    "<a class=\"anchor\" href=\"#{id}\" aria-label=\"Link to this section\">#</a></{tag}>\n"
                ))));
            }
            event => match &mut heading {
                Some((_, inner)) => inner.push(event),
                None => events.push(event),
            },
        }
    }

    flush(&mut events, &mut blocks);
    Ok(blocks)
}

fn split_info(info: &str) -> (&str, &str) {
    let info = info.trim();

    match info.find(|c: char| c.is_whitespace() || c == '{' || c == ',') {
        Some(at) => (&info[..at], info[at..].trim_start_matches(',').trim()),
        None => (info, ""),
    }
}

fn snippet(attributes: &str) -> Result<Snippet> {
    if attributes.is_empty() {
        return Ok(Snippet::default());
    }

    #[derive(Deserialize)]
    struct Wrapper {
        snippet: Snippet,
    }

    let wrapper: Wrapper = toml::from_str(&format!("snippet = {attributes}"))
        .with_context(|| format!("snippet attributes `{attributes}`"))?;

    Ok(wrapper.snippet)
}

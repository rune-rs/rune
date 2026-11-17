//! `site/home.toml`, the front page.

use anyhow::Result;
use rune_wasm::editor;
use rune_wasm::model;
use serde::Deserialize;

use crate::highlight::Highlighter;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Home {
    hero: Hero,
    features: Vec<Feature>,
    examples: Vec<Example>,
    safeguards: Showcase,
    embed: Showcase,
    closing: Closing,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hero {
    eyebrow: String,
    headline: String,
    lead: String,
    install: String,
    file: String,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Feature {
    icon: String,
    title: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Example {
    title: String,
    text: String,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Showcase {
    eyebrow: String,
    title: String,
    text: String,
    /// A runnable Rune program.
    source: Option<String>,
    /// Rust, shown highlighted.
    code: Option<String>,
    #[serde(default)]
    points: Vec<Point>,
    note: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    title: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Closing {
    title: String,
    text: String,
}

impl Home {
    pub fn render(self, highlighter: &Highlighter) -> Result<model::Home> {
        let showcase = |s: Showcase| model::Showcase {
            eyebrow: s.eyebrow,
            title: s.title,
            text: inline(&s.text),
            editor: s.source.as_deref().map(runnable),
            code: s.code.map(|code| highlighter.block("rust", &code)),
            points: s
                .points
                .into_iter()
                .map(|p| model::Point {
                    title: p.title,
                    text: inline(&p.text),
                })
                .collect(),
            note: s.note.as_deref().map(inline),
        };

        Ok(model::Home {
            hero: model::Hero {
                eyebrow: self.hero.eyebrow,
                headline: self.hero.headline,
                lead: inline(&self.hero.lead),
                install: self.hero.install,
                file: self.hero.file,
                editor: runnable(&self.hero.source),
            },
            features: self
                .features
                .into_iter()
                .map(|f| model::Feature {
                    icon: f.icon,
                    title: f.title,
                    text: inline(&f.text),
                })
                .collect(),
            examples: self
                .examples
                .into_iter()
                .map(|e| model::Example {
                    title: e.title,
                    text: inline(&e.text),
                    editor: runnable(&e.source),
                })
                .collect(),
            safeguards: showcase(self.safeguards),
            embed: showcase(self.embed),
            closing: model::Closing {
                title: self.closing.title,
                text: inline(&self.closing.text),
            },
        })
    }
}

fn runnable(source: &str) -> editor::Props {
    let (source, prelude) = editor::Props::snippet(source);

    editor::Props {
        source,
        prelude,
        run_on_change: true,
        ..editor::Props::default()
    }
}

/// A line of markdown, without the paragraph around it.
fn inline(markdown: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, pulldown_cmark::Parser::new(markdown));
    let html = html.trim();
    let html = html.strip_prefix("<p>").unwrap_or(html);
    let html = html.strip_suffix("</p>").unwrap_or(html);
    html.to_owned()
}

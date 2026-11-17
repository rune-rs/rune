//! The site's content as `tools/site` writes it to `/content` and the app
//! reads it back.

use serde::{Deserialize, Serialize};

use crate::editor;

/// `/content/site.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub title: String,
    pub description: String,
    pub tagline: String,
    pub links: Links,
    pub playground: String,
    /// The names of the pages under `/content/pages`, which are served at
    /// `/<name>/`.
    pub pages: Vec<String>,
    /// Newest first.
    pub posts: Vec<PostSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Links {
    pub book: String,
    pub docs: String,
    pub api: String,
    pub github: String,
    pub discord: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostSummary {
    pub slug: String,
    pub title: String,
    /// `2023-10-10`.
    pub date: String,
    /// `October 10, 2023`.
    pub date_long: String,
    pub author: Option<String>,
    pub minutes: u32,
    /// Rendered HTML.
    pub summary: String,
}

/// `/content/home.json`, `/content/pages/<name>.json` and
/// `/content/posts/<slug>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// Rendered markdown.
    Html { html: String },
    /// A runnable snippet.
    Editor {
        props: editor::Props,
        caption: Option<String>,
    },
}

/// `/content/home.json`, the front page. Every `text` is rendered HTML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Home {
    pub hero: Hero,
    pub features: Vec<Feature>,
    pub examples: Vec<Example>,
    pub safeguards: Showcase,
    pub embed: Showcase,
    pub closing: Closing,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hero {
    pub eyebrow: String,
    pub headline: String,
    pub lead: String,
    pub install: String,
    pub file: String,
    pub editor: editor::Props,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub icon: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Example {
    pub title: String,
    pub text: String,
    pub editor: editor::Props,
}

/// A section with text beside either a runnable editor or highlighted code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Showcase {
    pub eyebrow: String,
    pub title: String,
    pub text: String,
    pub editor: Option<editor::Props>,
    /// Rendered HTML.
    pub code: Option<String>,
    pub points: Vec<Point>,
    /// Said after the points, set apart from them.
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Closing {
    pub title: String,
    pub text: String,
}

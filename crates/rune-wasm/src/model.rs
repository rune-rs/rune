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

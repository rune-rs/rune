use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use lsp::Url;
use ropey::Rope;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

/// Shared server state.
#[derive(Clone)]
pub struct State {
    inner: Arc<Inner>,
}

impl State {
    /// Construct a new state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner::new()),
        }
    }

    /// Mark server as initialized.
    pub fn initialize(&self) {
        self.inner.initialized.store(true, Ordering::Release);
    }

    /// Test if server is initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.initialized.load(Ordering::Acquire)
    }

    /// Acess sources in the current state.
    pub async fn sources(&self) -> MutexGuard<'_, Sources> {
        self.inner.sources.lock().await
    }
}

struct Inner {
    /// Indicate if the server is initialized.
    initialized: AtomicBool,
    /// Sources used in the project.
    sources: Mutex<Sources>,
}

impl Inner {
    /// Construct a new empty inner state.
    pub fn new() -> Self {
        Self {
            initialized: Default::default(),
            sources: Default::default(),
        }
    }
}

/// A collection of open sources.
#[derive(Default)]
pub struct Sources {
    sources: HashMap<Url, Source>,
}

impl Sources {
    /// Insert the given source at the given url.
    pub fn insert_text(&mut self, url: Url, text: String) -> Option<Source> {
        let source = Source {
            content: Rope::from(text),
        };

        self.sources.insert(url, source)
    }

    /// Get the mutable source at the given url.
    pub fn get_mut(&mut self, url: &Url) -> Option<&mut Source> {
        self.sources.get_mut(url)
    }

    /// Remove the given url as a source.
    pub fn remove(&mut self, url: &Url) -> Option<Source> {
        self.sources.remove(url)
    }
}

/// A single open source.
pub struct Source {
    /// The content of the current source.
    content: Rope,
}

impl Source {
    /// Modify the given lsp range in the file.
    pub fn modify_lsp_range(&mut self, range: lsp::Range, content: &str) -> Result<()> {
        let start = rope_utf16_position(&self.content, range.start)?;
        let end = rope_utf16_position(&self.content, range.end)?;
        self.content.remove(start..end);

        if !content.is_empty() {
            self.content.insert(start, content);
        }

        Ok(())
    }

    /// Iterate over the text chunks in the source.
    pub fn chunks<'a>(&'a self) -> impl Iterator<Item = &'a str> {
        self.content.chunks()
    }

    /// Access the content of the source as a string.
    pub fn to_string(&self) -> String {
        self.content.to_string()
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

/// Translate the given lsp::Position, which is in UTF-16 because Microsoft.
///
/// Please go complain here:
/// https://github.com/microsoft/language-server-protocol/issues/376
fn rope_utf16_position(rope: &Rope, position: lsp::Position) -> Result<usize> {
    let line = rope.line(position.line as usize);

    // encoding target.
    let character = position.character as usize;

    let mut utf16_offset = 0usize;
    let mut char_offset = 0usize;

    for c in line.chars() {
        if utf16_offset == character {
            break;
        }

        if utf16_offset > character {
            return Err(anyhow!("character is not on an offset boundary"));
        }

        utf16_offset += c.len_utf16();
        char_offset += 1;
    }

    Ok(rope.line_to_char(position.line as usize) + char_offset)
}

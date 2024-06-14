//! A hot-reloader which watches a path for changes.
//!
//! This module is released under the public domain or [CC0] if your country
//! does not recognize public domain, or finally under the same license as Rune
//! which is MIT OR Apache 2.0.
//!
//! The gist of this is that while attribution is appreciated (thank you), it is
//! not required. You can copy, paste, and modify this code to your hearts
//! content without any need for attribution. It is provided as a basis for your
//! own projects.
//!
//! [CC0]: https://creativecommons.org/public-domain/cc0/

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::{pin, Pin};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use notify::Watcher;
use pin_project::pin_project;
use rune::termcolor::{ColorChoice, StandardStream};
use rune::{Context, Diagnostics, Source, Sources, Unit};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, Sleep};

/// The kind of path event emitted.
pub enum EventKind {
    /// The specified unit has been added.
    Added,
    /// The specified unit has been removed.
    Removed,
}

/// A path update event.
pub struct Event {
    /// The path that was modified.
    #[allow(unused)]
    pub path: PathBuf,
    /// The unit that was constructed from the path.
    pub unit: Arc<Unit>,
    /// The kind of event emitted.
    pub kind: EventKind,
}

impl Event {
    fn removed(path: PathBuf, unit: Arc<Unit>) -> Self {
        Self {
            path,
            unit,
            kind: EventKind::Removed,
        }
    }

    fn added(path: PathBuf, unit: Arc<Unit>) -> Self {
        Self {
            path,
            unit,
            kind: EventKind::Added,
        }
    }
}

enum Update {
    Updated,
    Removed,
}

/// A hot-reloader which watches a path for changes.
#[pin_project]
pub struct PathReloader<'a> {
    inner: Inner<'a>,
    rx: mpsc::UnboundedReceiver<notify::Result<notify::Event>>,
    #[pin]
    debounce: Sleep,
    _watcher: notify::RecommendedWatcher,
}

impl<'a> PathReloader<'a> {
    /// Construct a new path reloader for the specified directory.
    pub fn new<P>(path: P, context: &'a Context) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let (tx, rx) = mpsc::unbounded_channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            _ = tx.send(res);
        })?;

        watcher.watch(path.as_ref(), notify::RecursiveMode::NonRecursive)?;

        let mut this = Self {
            inner: Inner {
                context,
                path: path.as_ref().into(),
                scripts: Mutex::new(HashMap::new()),
                updates: HashMap::new(),
            },
            rx,
            debounce: tokio::time::sleep(Duration::from_secs(0)),
            _watcher: watcher,
        };

        this.initialize()?;
        Ok(this)
    }

    fn initialize(&mut self) -> Result<()> {
        for entry in fs::read_dir(self.inner.path.as_ref())? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) != Some("rn") {
                continue;
            }

            self.inner.updates.insert(path, Update::Updated);
        }

        Ok(())
    }

    /// Watch the current path for changes.
    pub async fn watch(self: Pin<&mut Self>, events: &mut Vec<Event>) -> Result<()> {
        let mut this = self.project();

        tokio::select! {
            _ = this.debounce.as_mut() => {
                this.inner.reload(events)?;
            }
            ev = this.rx.recv() => {
                let Some(ev) = ev.transpose()? else {
                    return Err(anyhow!("watcher closed"));
                };

                match ev.kind {
                    notify::EventKind::Remove(..) => {
                        for path in ev.paths {
                            this.inner.updates.insert(path, Update::Removed);
                        }
                    }
                    _ => {
                        for path in ev.paths {
                            this.inner.updates.insert(path, Update::Updated);
                        }
                    }
                }

                this.debounce.as_mut().reset(Instant::now() + Duration::from_millis(100));
            }
        }

        Ok(())
    }
}

struct Inner<'a> {
    context: &'a Context,
    path: Box<Path>,
    scripts: Mutex<HashMap<PathBuf, Arc<Unit>>>,
    updates: HashMap<PathBuf, Update>,
}

impl<'a> Inner<'a> {
    fn reload(&mut self, events: &mut Vec<Event>) -> Result<()> {
        fn compile(context: &Context, path: &Path) -> Result<Unit> {
            let mut sources = Sources::new();
            sources.insert(Source::from_path(path)?)?;

            let mut diagnostics = Diagnostics::new();

            let unit = rune::prepare(&mut sources)
                .with_diagnostics(&mut diagnostics)
                .with_context(context)
                .build();

            if !diagnostics.is_empty() {
                let mut writer = StandardStream::stderr(ColorChoice::Always);
                diagnostics.emit(&mut writer, &sources)?;
            }

            Ok(unit?)
        }

        for (path, update) in self.updates.drain() {
            match update {
                Update::Updated => {
                    let unit = match compile(self.context, &path) {
                        Ok(unit) => unit,
                        Err(error) => {
                            println!("{}: Failed to compile: {error}", path.display());

                            if let Some(old) = self.scripts.lock().unwrap().remove(&path) {
                                events.push(Event::removed(path.clone(), old));
                            }

                            continue;
                        }
                    };

                    let new = Arc::new(unit);

                    if let Some(old) = self
                        .scripts
                        .lock()
                        .unwrap()
                        .insert(path.clone(), new.clone())
                    {
                        events.push(Event::removed(path.clone(), old));
                    }

                    events.push(Event::added(path, new));
                }
                Update::Removed => {
                    if let Some(unit) = self.scripts.lock().unwrap().remove(&path) {
                        events.push(Event::removed(path, unit));
                    }
                }
            }
        }

        Ok(())
    }
}

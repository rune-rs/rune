//! Helper to generate documentation from a context.

mod templating;

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

use anyhow::{anyhow, bail, Context as _, Error, Result};
use relative_path::{RelativePath, RelativePathBuf};
use rust_embed::RustEmbed;
use serde::{Serialize, Serializer};
use syntect::highlighting::ThemeSet;
use syntect::html::{self, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::compile::{ComponentRef, ContextMetaKind, ContextSignature, Docs, Item, ItemBuf};

const RUST_TOKEN: &str = "rust";
const RUNE_TOKEN: &str = "rune";

// InspiredGitHub
// Solarized (dark)
// Solarized (light)
// base16-eighties.dark
// base16-mocha.dark
// base16-ocean.dark
// base16-ocean.light
const THEME: &str = "base16-eighties.dark";

#[derive(RustEmbed)]
#[folder = "src/doc/static"]
struct Assets;

#[derive(Serialize)]
struct Shared {
    fonts: Vec<RelativePathBuf>,
    css: Vec<RelativePathBuf>,
}

struct Ctxt<'a> {
    fonts: &'a [RelativePathBuf],
    css: &'a [RelativePathBuf],
    index_template: templating::Template,
    module_template: templating::Template,
    type_template: templating::Template,
    struct_template: templating::Template,
    syntax_set: SyntaxSet,
}

impl Ctxt<'_> {
    fn shared(&self, dir: &RelativePath) -> Shared {
        Shared {
            fonts: self.fonts.iter().map(|f| dir.relative(f)).collect(),
            css: self.css.iter().map(|f| dir.relative(f)).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Build {
    Type(ItemBuf),
    Struct(ItemBuf),
    Module(ItemBuf),
}

/// Compile a template.
fn compile(templating: &templating::Templating, path: &str) -> Result<templating::Template> {
    let template = Assets::get(path).with_context(|| anyhow!("{path}: missing"))?;
    let template = std::str::from_utf8(template.data.as_ref())
        .with_context(|| anyhow!("{path}: not utf-8"))?;
    templating.compile(template)
}

/// Write html documentation to the given path.
pub fn write_html(context: &crate::Context, root: &Path) -> Result<()> {
    // Collect an ordered set of modules, so we have a baseline of what to render when.
    let mut modules = BTreeSet::new();

    let templating = templating::Templating::new()?;

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let mut fonts = Vec::new();
    let mut css = Vec::new();

    for file in Assets::iter() {
        let path = RelativePath::new(file.as_ref());

        match (path.file_name(), path.extension()) {
            (Some(name), Some("woff2")) => {
                let file = Assets::get(file.as_ref()).context("missing font")?;
                let path = RelativePath::new(name);
                tracing::info!("writing: {}", path);
                fs::write(path.to_path(root), file.data.as_ref())
                    .with_context(|| path.to_owned())?;
                fonts.push(path.to_owned());
            }
            (Some(name), Some("css")) => {
                let file = Assets::get(file.as_ref()).context("missing font")?;
                let path = RelativePath::new(name);
                tracing::info!("writing: {}", path);
                fs::write(path.to_path(root), file.data.as_ref())
                    .with_context(|| path.to_owned())?;
                css.push(path.to_owned());
            }
            _ => {}
        }
    }

    let syntax_css = RelativePath::new("syntax.css");
    let theme = theme_set.themes.get(THEME).context("missing theme")?;
    let syntax_css_content = html::css_for_theme_with_class_style(theme, html::ClassStyle::Spaced)?;
    tracing::info!("writing: {}", syntax_css);
    fs::write(syntax_css.to_path(root), syntax_css_content)
        .with_context(|| syntax_css.to_owned())?;
    css.push(syntax_css.to_owned());

    let cx = Ctxt {
        fonts: &fonts,
        css: &css,
        index_template: compile(&templating, "index.html.hbs")?,
        module_template: compile(&templating, "module.html.hbs")?,
        type_template: compile(&templating, "type.html.hbs")?,
        struct_template: compile(&templating, "struct.html.hbs")?,
        syntax_set,
    };

    for (_, meta) in context.iter_meta() {
        modules.insert(Build::Module(meta.module.clone()));
    }

    let mut queue = modules.into_iter().collect::<VecDeque<_>>();

    let mut modules = Vec::new();

    while let Some(build) = queue.pop_front() {
        match build {
            Build::Type(m) => {
                type_(&cx, context, &m, root)?;
            }
            Build::Struct(m) => {
                struct_(&cx, context, &m, root)?;
            }
            Build::Module(m) => {
                let path = module(&cx, context, &m, root, &mut queue)?;
                modules.push((m, path));
            }
        }
    }

    index(&cx, root, &modules)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
fn index(cx: &Ctxt<'_>, root: &Path, mods: &[(ItemBuf, RelativePathBuf)]) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        modules: &'a [Module<'a>],
    }

    #[derive(Serialize)]
    struct Module<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        path: &'a RelativePath,
    }

    let p = root.join("index.html");
    let dir = RelativePath::new("");

    let mut modules = Vec::new();

    for (item, path) in mods {
        modules.push(Module { item, path });
    }

    let data = cx.index_template.render(&Params {
        shared: cx.shared(dir),
        modules: &modules,
    })?;

    tracing::info!("writing: {}", p.display());
    fs::write(&p, data).with_context(|| p.display().to_string())?;
    Ok(())
}

/// Build a single module.
#[tracing::instrument(skip_all)]
fn module(
    cx: &Ctxt<'_>,
    context: &crate::Context,
    m: &Item,
    root: &Path,
    queue: &mut VecDeque<Build>,
) -> Result<RelativePathBuf> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        types: Vec<Type<'a>>,
        structs: Vec<Struct<'a>>,
        functions: Vec<Function<'a>>,
    }

    #[derive(Serialize)]
    struct Type<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
        first: Option<&'a str>,
    }

    #[derive(Serialize)]
    struct Struct<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
        first: Option<&'a str>,
    }

    #[derive(Serialize)]
    struct Function<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
    }

    let path = item_path(m, ItemPath::Module)?;
    let dir = path.parent().unwrap_or(RelativePath::new(""));

    let mut types = Vec::new();
    let mut structs = Vec::new();
    let mut functions = Vec::new();

    for name in context.iter_components(m) {
        let m = m.join([name]);

        let (meta, f) = match (context.lookup_meta(&m), context.lookup_function_info(&m)) {
            (Some(meta), f) => (meta, f),
            _ => continue,
        };

        match (&meta.kind, f) {
            (ContextMetaKind::Unknown { .. }, _) => {
                let path = dir.relative(item_path(&m, ItemPath::Type)?);
                types.push(Type {
                    path,
                    item: &meta.item,
                    name,
                    first: meta.docs.lines().next(),
                });
                queue.push_front(Build::Type(m));
            }
            (ContextMetaKind::Struct { .. }, _) => {
                let path = dir.relative(item_path(&m, ItemPath::Struct)?);
                structs.push(Struct {
                    path,
                    item: &meta.item,
                    name,
                    first: meta.docs.lines().next(),
                });
                queue.push_front(Build::Struct(m));
            }
            (ContextMetaKind::Variant { .. }, _) => {}
            (ContextMetaKind::Enum { .. }, _) => {}
            (
                ContextMetaKind::Function {
                    instance_function, ..
                },
                Some(f),
            ) => {
                if !matches!(f, ContextSignature::Instance { .. }) && !instance_function {
                    let doc = cx.render_docs(&meta.docs)?;

                    functions.push(Function {
                        item: &meta.item,
                        name,
                        args: args_to_string(meta.docs.args(), f)?,
                        doc,
                    });
                }
            }
            (ContextMetaKind::Function { .. }, None) => {}
            (ContextMetaKind::Const { .. }, _) => {}
        }
    }

    let p = path.to_path(root);
    ensure_parent_dir(&p)?;

    let data = cx.module_template.render(&Params {
        shared: cx.shared(dir),
        item: m,
        types,
        structs,
        functions,
    })?;

    tracing::info!("writing: {}", p.display());
    fs::write(&p, data).with_context(|| p.display().to_string())?;
    Ok(path)
}

/// Build an unknown type.
#[tracing::instrument(skip_all)]
fn type_(
    cx: &Ctxt<'_>,
    context: &crate::Context,
    m: &Item,
    root: &Path,
) -> Result<RelativePathBuf> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        methods: Vec<Method<'a>>,
    }

    #[derive(Serialize)]
    struct Method<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
    }

    let path = item_path(m, ItemPath::Type)?;
    let dir = path.parent().unwrap_or(RelativePath::new(""));

    let mut methods = Vec::new();

    for name in context.iter_components(m) {
        let m = m.join([name]);

        let (meta, f) = match (context.lookup_meta(&m), context.lookup_function_info(&m)) {
            (Some(meta), f) => (meta, f),
            _ => continue,
        };

        match (&meta.kind, f) {
            (ContextMetaKind::Function { .. }, Some(f)) => {
                let doc = cx.render_docs(&meta.docs)?;

                methods.push(Method {
                    item: &meta.item,
                    name,
                    args: args_to_string(meta.docs.args(), f)?,
                    doc,
                });
            }
            _ => {
                continue;
            }
        }
    }

    let p = path.to_path(root);
    ensure_parent_dir(&p)?;

    let data = cx.type_template.render(&Params {
        shared: cx.shared(dir),
        item: m,
        methods,
    })?;

    tracing::info!("writing: {}", p.display());
    fs::write(&p, data).with_context(|| p.display().to_string())?;
    Ok(path)
}

/// Build a single struct.
#[tracing::instrument(skip_all)]
fn struct_(
    cx: &Ctxt<'_>,
    context: &crate::Context,
    m: &Item,
    root: &Path,
) -> Result<RelativePathBuf> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        methods: Vec<Method<'a>>,
    }

    #[derive(Serialize)]
    struct Method<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
    }

    let path = item_path(m, ItemPath::Struct)?;
    let dir = path.parent().unwrap_or(RelativePath::new(""));

    let mut methods = Vec::new();

    for name in context.iter_components(m) {
        let m = m.join([name]);

        let (meta, f) = match (context.lookup_meta(&m), context.lookup_function_info(&m)) {
            (Some(meta), f) => (meta, f),
            _ => continue,
        };

        match (&meta.kind, f) {
            (ContextMetaKind::Function { .. }, Some(f)) => {
                let doc = cx.render_docs(&meta.docs)?;

                methods.push(Method {
                    item: &meta.item,
                    name,
                    args: args_to_string(meta.docs.args(), f)?,
                    doc,
                });
            }
            _ => {
                continue;
            }
        }
    }

    let p = path.to_path(root);
    ensure_parent_dir(&p)?;

    let data = cx.struct_template.render(&Params {
        shared: cx.shared(dir),
        item: m,
        methods,
    })?;

    tracing::info!("writing: {}", p.display());
    fs::write(&p, data).with_context(|| p.display().to_string())?;
    Ok(path)
}

enum ItemPath {
    Type,
    Struct,
    Module,
}

fn item_path(item: &Item, kind: ItemPath) -> Result<RelativePathBuf> {
    let mut path = RelativePathBuf::new();

    for c in item.iter() {
        let string = match c {
            ComponentRef::Crate(string) => string,
            ComponentRef::Str(string) => string,
            _ => continue,
        };

        path.push(string);
    }

    let ext = match kind {
        ItemPath::Type => "type.html",
        ItemPath::Struct => "struct.html",
        ItemPath::Module => "module.html",
    };

    if path.file_name().is_none() {
        bail!("missing file name ({ext})");
    }

    path.set_extension(ext);
    Ok(path)
}

impl Ctxt<'_> {
    /// Render documentation.
    fn render_docs(&self, docs: &Docs) -> Result<Option<String>> {
        use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag};
        use std::fmt::Write;

        struct Filter<'a> {
            cx: &'a Ctxt<'a>,
            parser: Parser<'a, 'a>,
            codeblock: Option<&'a SyntaxReference>,
        }

        impl<'a> Filter<'a> {
            fn new(cx: &'a Ctxt<'a>, parser: Parser<'a, 'a>) -> Self {
                Self {
                    cx,
                    parser,
                    codeblock: None,
                }
            }
        }

        impl<'a> Iterator for Filter<'a> {
            type Item = Event<'a>;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                let e = self.parser.next()?;

                match (e, self.codeblock) {
                    (Event::Start(Tag::CodeBlock(kind)), _) => {
                        self.codeblock = None;

                        if let CodeBlockKind::Fenced(fences) = &kind {
                            for token in fences.split(',') {
                                let token = match token.trim() {
                                    RUNE_TOKEN => RUST_TOKEN,
                                    token => token,
                                };

                                if let Some(syntax) = self.cx.syntax_set.find_syntax_by_token(token)
                                {
                                    self.codeblock = Some(syntax);
                                    return Some(Event::Start(Tag::CodeBlock(kind)));
                                }
                            }
                        }

                        if self.codeblock.is_none() {
                            self.codeblock = self.cx.syntax_set.find_syntax_by_token(RUST_TOKEN);
                        }

                        if self.codeblock.is_none() {
                            self.codeblock = Some(self.cx.syntax_set.find_syntax_plain_text());
                        }

                        Some(Event::Start(Tag::CodeBlock(kind)))
                    }
                    (Event::End(Tag::CodeBlock(kind)), Some(_)) => {
                        self.codeblock = None;
                        Some(Event::End(Tag::CodeBlock(kind)))
                    }
                    (Event::Text(text), syntax) => {
                        if let Some(syntax) = syntax {
                            let mut buf = String::new();

                            let mut gen = ClassedHTMLGenerator::new_with_class_style(
                                syntax,
                                &self.cx.syntax_set,
                                ClassStyle::Spaced,
                            );

                            for line in text.lines() {
                                let line = line.strip_prefix(' ').unwrap_or(line);

                                if line.starts_with('#') {
                                    continue;
                                }

                                buf.clear();
                                buf.push_str(line);
                                buf.push('\n');
                                let _ = gen.parse_html_for_line_which_includes_newline(&buf);
                            }

                            let html = gen.finalize();
                            Some(Event::Html(CowStr::Boxed(html.into())))
                        } else {
                            let mut buf = String::new();

                            for line in text.lines() {
                                let line = line.strip_prefix(' ').unwrap_or(line);

                                if line.starts_with('#') {
                                    continue;
                                }

                                buf.push_str(line);
                                buf.push('\n');
                            }

                            Some(Event::Text(CowStr::Boxed(buf.into())))
                        }
                    }
                    (event, _) => Some(event),
                }
            }
        }

        if docs.is_empty() {
            return Ok(None);
        }

        let mut o = String::new();
        write!(o, "<div class=\"docs\">")?;
        let mut input = String::new();

        for line in docs.lines() {
            let line = line.strip_prefix(' ').unwrap_or(line);
            input.push_str(line);
            input.push('\n');
        }

        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Filter::new(self, Parser::new_ext(&input, options));
        let mut out = String::new();
        pulldown_cmark::html::push_html(&mut out, parser);
        write!(o, "{out}")?;
        write!(o, "</div>")?;
        Ok(Some(o))
    }
}

/// Helper to serialize an item.
fn serialize_item<S>(item: &Item, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(item)
}

/// Helper to serialize a component ref.
fn serialize_component_ref<S>(c: &ComponentRef<'_>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&c)
}

/// Ensure parent dir exists.
fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(p) = path.parent() {
        if p.is_dir() {
            return Ok(());
        }

        tracing::info!("create dir: {}", p.display());

        match fs::create_dir_all(p) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(Error::from(e)).context(p.display().to_string()),
        }
    }

    Ok(())
}

/// Coerce args into string.
fn args_to_string(args: Option<&[String]>, sig: &ContextSignature) -> Result<String> {
    use std::fmt::Write;

    if let Some(args) = args {
        return Ok(args.join(", "));
    }

    let mut string = String::new();

    match sig {
        ContextSignature::Function { args, .. } => {
            let mut string = String::new();

            if let Some(count) = args {
                let mut it = 0..*count;
                let last = it.next_back();

                for n in it {
                    write!(string, "arg{n}, ")?;
                }

                if let Some(n) = last {
                    write!(string, "arg{n}")?;
                }
            } else {
                write!(string, "..")?;
            }

            Ok(string)
        }
        ContextSignature::Instance { args, .. } => {
            write!(string, "self")?;

            match args {
                Some(n) => {
                    for n in 0..*n {
                        write!(string, ", arg{n}")?;
                    }
                }
                None => {
                    write!(string, ", ..")?;
                }
            }

            Ok(string)
        }
    }
}

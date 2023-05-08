mod enum_;
mod type_;
mod markdown;
mod js;

use std::fmt::{self, Write};
use std::fs;
use std::io;
use std::path::Path;
use std::str;

use crate::no_std::prelude::*;
use crate::no_std::borrow::Cow;

use anyhow::{anyhow, bail, Context as _, Error, Result};
use relative_path::{RelativePath, RelativePathBuf};
use rust_embed::EmbeddedFile;
use rust_embed::RustEmbed;
use serde::{Serialize, Serializer};
use syntect::highlighting::ThemeSet;
use syntect::html::{self, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::collections::{BTreeSet, VecDeque};
use crate::compile::{ComponentRef, Item, ItemBuf};
use crate::doc::context::{Function, Kind, Signature};
use crate::doc::templating;
use crate::doc::{Context, Visitor};
use crate::Hash;

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

/// Write html documentation to the given path.
pub fn write_html(
    name: &str,
    root: &Path,
    context: &crate::Context,
    visitors: &[Visitor],
) -> Result<()> {
    let context = Context::new(context, visitors);

    let templating = templating::Templating::new([
        ("layout", asset_str("layout.html.hbs")?),
    ])?;

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let mut fonts = Vec::new();
    let mut css = Vec::new();
    let mut js = Vec::new();

    for file in Assets::iter() {
        let path = RelativePath::new(file.as_ref());

        match (path.file_name(), path.extension()) {
            (Some(name), Some("woff2")) => {
                let file = Assets::get(file.as_ref()).context("missing asset")?;
                let path = copy_file(name, root, file)?;
                fonts.push(path.to_owned());
            }
            (Some(name), Some("css")) => {
                let file = Assets::get(file.as_ref()).context("missing asset")?;
                let path = copy_file(name, root, file)?;
                css.push(path.to_owned());
            }
            (Some(name), Some("js")) => {
                let file = Assets::get(file.as_ref()).context("missing asset")?;
                let path = copy_file(name, root, file)?;
                js.push(path.to_owned());
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

    // Collect an ordered set of modules, so we have a baseline of what to render when.
    let mut initial = BTreeSet::new();

    for module in context.iter_modules() {
        let hash = Hash::type_hash(&module);
        initial.insert(Build::Module(Cow::Owned(module), hash));
    }

    let search_index = RelativePath::new("index.js");

    let mut cx = Ctxt {
        root,
        path: RelativePathBuf::new(),
        item: ItemBuf::new(),
        item_kind: ItemKind::Module,
        index: Vec::new(),
        name,
        context: &context,
        search_index: Some(search_index),
        fonts: &fonts,
        css: &css,
        js: &js,
        index_template: compile(&templating, "index.html.hbs")?,
        module_template: compile(&templating, "module.html.hbs")?,
        type_template: compile(&templating, "type.html.hbs")?,
        macro_template: compile(&templating, "macro.html.hbs")?,
        function_template: compile(&templating, "function.html.hbs")?,
        enum_template: compile(&templating, "enum.html.hbs")?,
        syntax_set,
    };

    let mut queue = initial.into_iter().collect::<VecDeque<_>>();

    let mut modules = Vec::new();

    while let Some(build) = queue.pop_front() {
        match build {
            Build::Type(item, hash) => {
                cx.set_path(item, hash, ItemKind::Type)?;
                let items = self::type_::build(&cx, "Type", "type", hash)?;
                cx.index.extend(items);
            }
            Build::Struct(item, hash) => {
                cx.set_path(item, hash, ItemKind::Struct)?;
                let items = self::type_::build(&cx, "Struct", "struct", hash)?;
                cx.index.extend(items);
            }
            Build::Enum(item, hash) => {
                cx.set_path(item, hash, ItemKind::Enum)?;
                self::enum_::build(&cx, hash)?;
            }
            Build::Macro(item, hash) => {
                cx.set_path(item, hash, ItemKind::Macro)?;
                build_macro(&cx)?;
            }
            Build::Function(item, hash) => {
                cx.set_path(item, hash, ItemKind::Function)?;
                build_function(&cx)?;
            }
            Build::Module(item, hash) => {
                cx.set_path(item.as_ref(), hash, ItemKind::Module)?;
                module(&cx, hash, &mut queue)?;
                modules.push((item, cx.path.clone()));
            }
        }
    }

    cx.path = RelativePath::new("index.html").to_owned();
    index(&cx, &modules)?;

    cx.path = search_index.to_owned();
    cx.write_file(|cx| {
        let mut s = String::new();
        write!(s, "window.INDEX = [")?;

        let mut it = cx.index.iter();

        while let Some(IndexEntry { path, item, kind, doc }) = it.next() {
            write!(s, "[\"{path}\",\"{item}\",\"{kind}\",\"")?;

            if let Some(doc) = doc {
                js::encode_quoted(&mut s, doc);
            }

            write!(s, "\"]")?;

            if it.clone().next().is_some() {
                write!(s, ",")?;
            }
        }

        write!(s, "];")?;
        writeln!(s)?;
        Ok(s)
    })?;

    Ok(())
}

#[derive(Serialize)]
struct Shared<'a> {
    path: Option<&'a RelativePath>,
    search_index: Option<RelativePathBuf>,
    fonts: Vec<RelativePathBuf>,
    css: Vec<RelativePathBuf>,
    js: Vec<RelativePathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ItemKind {
    Type,
    Struct,
    Enum,
    Module,
    Macro,
    Function,
}

impl ItemKind {
    fn matches_kind(&self, kind: &Kind<'_>) -> bool {
        match (self, kind) {
            (ItemKind::Type, Kind::Type) => true,
            (ItemKind::Struct, Kind::Struct) => true,
            (ItemKind::Enum, Kind::Enum) => true,
            (ItemKind::Module, Kind::Module) => true,
            (ItemKind::Macro, Kind::Macro) => true,
            (ItemKind::Function, Kind::Function(..)) => true,
            _ => false,
        }
    }
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ItemKind::Type => "type".fmt(f),
            ItemKind::Struct => "struct".fmt(f),
            ItemKind::Enum => "enum".fmt(f),
            ItemKind::Module => "module".fmt(f),
            ItemKind::Macro => "macro".fmt(f),
            ItemKind::Function => "function".fmt(f),
        }
    }
}

pub(crate) enum IndexKind {
    Item(ItemKind),
    Method,
}

impl fmt::Display for IndexKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexKind::Item(item) => item.fmt(f),
            IndexKind::Method => "method".fmt(f),
        }
    }
}

pub(crate) struct IndexEntry {
    pub(crate) path: RelativePathBuf,
    pub(crate) item: ItemBuf,
    pub(crate) kind: IndexKind,
    pub(crate) doc: Option<String>,
}

pub(crate) struct Ctxt<'a, 'm> {
    root: &'a Path,
    path: RelativePathBuf,
    item: ItemBuf,
    item_kind: ItemKind,
    /// A collection of all items visited.
    index: Vec<IndexEntry>,
    name: &'a str,
    context: &'a Context<'m>,
    search_index: Option<&'a RelativePath>,
    fonts: &'a [RelativePathBuf],
    css: &'a [RelativePathBuf],
    js: &'a [RelativePathBuf],
    index_template: templating::Template,
    module_template: templating::Template,
    type_template: templating::Template,
    macro_template: templating::Template,
    function_template: templating::Template,
    enum_template: templating::Template,
    syntax_set: SyntaxSet,
}

impl Ctxt<'_, '_> {
    fn set_path(&mut self, item: &Item, hash: Hash, kind: ItemKind) -> Result<()> {
        self.path = RelativePathBuf::new();
        build_item_path(self.name, item, kind, &mut self.path)?;
        self.item = item.to_owned();
        self.item_kind = kind;

        let doc = match self.context.meta_by_hash(hash).into_iter().find(|m| kind.matches_kind(&m.kind)) {
            Some(m) => self.render_docs(m.docs.get(..1).unwrap_or_default())?,
            None => None,
        };

        self.index.push(IndexEntry {
            path: self.path.clone(),
            item: self.item.clone(),
            kind: IndexKind::Item(self.item_kind),
            doc,
        });

        Ok(())
    }

    fn dir(&self) -> &RelativePath {
        self.path.parent().unwrap_or(RelativePath::new(""))
    }

    fn shared(&self) -> Shared<'_> {
        let dir = self.dir();

        Shared {
            path: self.path.parent(),
            search_index: self.search_index.map(|p| dir.relative(p)),
            fonts: self.fonts.iter().map(|f| dir.relative(f)).collect(),
            css: self.css.iter().map(|f| dir.relative(f)).collect(),
            js: self.js.iter().map(|f| dir.relative(f)).collect(),
        }
    }

    /// Write the current file.
    fn write_file<C>(&self, contents: C) -> Result<()>
    where
        C: FnOnce(&Self) -> Result<String>,
    {
        let p = self.path.to_path(self.root);
        tracing::info!("writing: {}", p.display());
        ensure_parent_dir(&p)?;
        let data = contents(self)?;
        fs::write(&p, data).with_context(|| p.display().to_string())?;
        Ok(())
    }

    /// Render rust code.
    fn render_code<I>(&self, lines: I) -> Result<String>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let syntax = match self.syntax_set.find_syntax_by_token(self::markdown::RUST_TOKEN) {
            Some(syntax) => syntax,
            None => self.syntax_set.find_syntax_plain_text(),
        };

        Ok(format!(
            "<pre><code class=\"language-rune\">{}</code></pre>",
            render_code_by_syntax(&self.syntax_set, lines, syntax)?
        ))
    }

    /// Render documentation.
    fn render_docs<S>(&self, docs: &[S]) -> Result<Option<String>>
    where
        S: AsRef<str>,
    {
        use pulldown_cmark::{Options, Parser, BrokenLink};
        use std::fmt::Write;

        if docs.is_empty() {
            return Ok(None);
        }

        let mut input = String::new();

        for line in docs {
            let line = line.as_ref();
            let line = line.strip_prefix(' ').unwrap_or(line);
            input.push_str(line);
            input.push('\n');
        }

        let mut o = String::new();
        write!(o, "<div class=\"docs\">")?;
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let mut callback = |link: BrokenLink<'_>| {
            let (path, title) = self.link_callback(link.reference.as_ref())?;
            Some((path.to_string().into(), title.into()))
        };

        let iter = Parser::new_with_broken_link_callback(&input, options, Some(&mut callback));

        markdown::push_html(&self.syntax_set, &mut o, iter)?;
        write!(o, "</div>")?;
        Ok(Some(o))
    }

    #[inline]
    fn item_path(&self, item: &Item, kind: ItemKind) -> Result<RelativePathBuf> {
        let mut path = RelativePathBuf::new();
        build_item_path(self.name, item, kind, &mut path)?;
        Ok(self.dir().relative(path))
    }

    /// Build banklinks for the current item.
    fn module_path_html(&self, is_module: bool) -> Result<String> {
        let mut module = Vec::new();
        let mut iter = self.item.iter();

        while iter.next_back().is_some() {
            if let Some(name) = iter.as_item().last() {
                let url = self.item_path(iter.as_item(), ItemKind::Module)?;
                module.push(format!("<a class=\"module\" href=\"{url}\">{name}</a>"));
            }
        }

        module.reverse();

        if is_module {
            if let Some(name) = self.item.last() {
                module.push(format!("<span class=\"module\">{name}</span>"));
            }
        }

        Ok(module.join("::"))
    }

    /// Convert a hash into a link.
    fn link(&self, hash: Hash, text: Option<&str>) -> Result<String> {
        let link = if let [meta] = self.context.meta_by_hash(hash).as_slice() {
            let name = match text {
                Some(text) => text,
                None => meta
                    .item
                    .last()
                    .and_then(|c| c.as_str())
                    .context("missing name")?,
            };

            match &meta.kind {
                Kind::Type => {
                    let path = self.item_path(meta.item, ItemKind::Type)?;
                    format!("<a class=\"type\" href=\"{path}\">{name}</a>")
                }
                Kind::Struct => {
                    let path = self.item_path(meta.item, ItemKind::Struct)?;
                    format!("<a class=\"struct\" href=\"{path}\">{name}</a>")
                }
                Kind::Enum => {
                    let path = self.item_path(meta.item, ItemKind::Enum)?;
                    format!("<a class=\"enum\" href=\"{path}\">{name}</a>")
                }
                kind => format!("{kind:?}"),
            }
        } else {
            String::from("<b>n/a</b>")
        };

        Ok(link)
    }

    /// Coerce args into string.
    fn args_to_string(
        &self,
        args: Option<&[String]>,
        sig: Signature,
        argument_types: &[Option<Hash>],
    ) -> Result<String> {
        use std::borrow::Cow;
        use std::fmt::Write;

        let mut string = String::new();
        let mut types = argument_types.iter();

        let mut args_iter;
        let mut function_iter;
        let mut instance_iter;

        let it: &mut dyn Iterator<Item = Cow<'_, str>> = if let Some(args) = args {
            args_iter = args.iter().map(|s| Cow::Borrowed(s.as_str()));
            &mut args_iter
        } else {
            match sig {
                Signature::Function { args, .. } => {
                    let mut string = String::new();

                    let Some(count) = args else {
                        write!(string, "..")?;
                        return Ok(string);
                    };

                    function_iter = (0..count).map(|n| {
                        if n == 0 {
                            Cow::Borrowed("value")
                        } else {
                            Cow::Owned(format!("value{}", n))
                        }
                    });

                    &mut function_iter
                }
                Signature::Instance { args, .. } => {
                    let s = [Cow::Borrowed("self")];

                    let (n, f): (usize, fn(usize) -> Cow<'static, str>) = match args {
                        Some(n) => {
                            let f = move |n| {
                                if n != 1 {
                                    Cow::Owned(format!("value{n}"))
                                } else {
                                    Cow::Borrowed("value")
                                }
                            };

                            (n, f)
                        }
                        None => {
                            let f = move |_| Cow::Borrowed("..");
                            (2, f)
                        }
                    };

                    instance_iter = s.into_iter().chain((1..n).map(f));
                    &mut instance_iter
                }
            }
        };

        let mut it = it.peekable();

        while let Some(arg) = it.next() {
            if arg == "self" {
                if let Some(Some(hash)) = types.next() {
                    string.push_str(&self.link(*hash, Some("self"))?);
                } else {
                    string.push_str("self");
                }
            } else {
                string.push_str(arg.as_ref());

                if let Some(Some(hash)) = types.next() {
                    string.push_str(": ");
                    string.push_str(&self.link(*hash, None)?);
                }
            }

            if it.peek().is_some() {
                write!(string, ", ")?;
            }
        }

        Ok(string)
    }

    fn link_callback(&self, link: &str) -> Option<(RelativePathBuf, String)> {
        enum Flavor {
            Any,
            Macro,
            Function,
        }

        impl Flavor {
            fn is_struct(&self) -> bool {
                matches!(self, Flavor::Any)
            }

            fn is_enum(&self) -> bool {
                matches!(self, Flavor::Any)
            }

            fn is_macro(&self) -> bool {
                matches!(self, Flavor::Any | Flavor::Macro)
            }

            fn is_function(&self) -> bool {
                matches!(self, Flavor::Any | Flavor::Function)
            }
        }

        fn flavor(link: &str) -> (&str, Flavor) {
            if let Some(link) = link.strip_suffix('!') {
                return (link, Flavor::Macro);
            }

            if let Some(link) = link.strip_suffix("()") {
                return (link, Flavor::Function);
            }

            (link, Flavor::Any)
        }

        let link = link.trim_matches(|c| matches!(c, '`'));
        let (link, flavor) = flavor(link);

        let item = if matches!(self.item_kind, ItemKind::Module) {
            self.item.join([link])
        } else {
            self.item.parent()?.join([link])
        };

        let item_path = 'out: {
            let mut alts = Vec::new();

            for meta in self.context.meta(&item) {
                alts.push(match meta.kind {
                    Kind::Struct if flavor.is_struct() => ItemKind::Struct,
                    Kind::Enum if flavor.is_enum() => ItemKind::Enum,
                    Kind::Macro if flavor.is_macro() => ItemKind::Macro,
                    Kind::Function(_) if flavor.is_function() => ItemKind::Function,
                    _ => {
                        continue;
                    },
                });
            }

            match &alts[..] {
                [] => {
                    tracing::warn!(?link, "Bad link, no items found");
                }
                [out] => break 'out *out,
                items => {
                    tracing::warn!(?link, ?items, "Bad link, got multiple items");
                }
            }

            return None;
        };

        let path = self.item_path(&item, item_path).ok()?;
        let title = format!("{item_path} {link}");
        Some((path, title))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Build<'a> {
    Type(&'a Item, Hash),
    Struct(&'a Item, Hash),
    Enum(&'a Item, Hash),
    Macro(&'a Item, Hash),
    Function(&'a Item, Hash),
    Module(Cow<'a, Item>, Hash),
}

/// Get an asset as a string.
fn asset_str(path: &str) -> Result<Cow<'static, str>> {
    let asset = Assets::get(path).with_context(|| anyhow!("{path}: missing asset"))?;

    let data = match asset.data {
        Cow::Borrowed(data) => Cow::Borrowed(str::from_utf8(data).with_context(|| anyhow!("{path}: not utf-8"))?),
        Cow::Owned(data) => Cow::Owned(String::from_utf8(data).with_context(|| anyhow!("{path}: not utf-8"))?),
    };

    Ok(data)
}

/// Compile a template.
fn compile(templating: &templating::Templating, path: &str) -> Result<templating::Template> {
    let template = asset_str(path)?;
    templating.compile(template.as_ref())
}

/// Copy an embedded file.
fn copy_file<'a>(
    name: &'a str,
    root: &Path,
    file: EmbeddedFile,
) -> Result<&'a RelativePath, Error> {
    let path = RelativePath::new(name);
    let file_path = path.to_path(root);
    tracing::info!("writing: {}", file_path.display());
    ensure_parent_dir(&file_path)?;
    fs::write(&file_path, file.data.as_ref()).with_context(|| file_path.display().to_string())?;
    Ok(path)
}

#[tracing::instrument(skip_all)]
fn index(cx: &Ctxt<'_, '_>, mods: &[(Cow<'_, Item>, RelativePathBuf)]) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        modules: &'a [Module<'a>],
    }

    #[derive(Serialize)]
    struct Module<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        path: &'a RelativePath,
    }

    let mut modules = Vec::new();

    for (item, path) in mods {
        let mut c = item.iter();

        match c.next() {
            None => {},
            Some(ComponentRef::Crate(..)) => {}
            _ => continue,
        }

        if c.next().is_some() {
            continue;
        }

        modules.push(Module { item, path });
    }

    cx.write_file(|cx| {
        cx.index_template.render(&Params {
            shared: cx.shared(),
            modules: &modules,
        })
    })
}

/// Build a single module.
#[tracing::instrument(skip_all)]
fn module<'m>(cx: &Ctxt<'_, 'm>, hash: Hash, queue: &mut VecDeque<Build<'m>>) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        module: String,
        doc: Option<String>,
        types: Vec<Type<'a>>,
        structs: Vec<Struct<'a>>,
        enums: Vec<Enum<'a>>,
        macros: Vec<Macro<'a>>,
        functions: Vec<Function<'a>>,
        modules: Vec<Module<'a>>,
    }

    #[derive(Serialize)]
    struct Type<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
        first: Option<&'a String>,
    }

    #[derive(Serialize)]
    struct Struct<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        first: Option<&'a String>,
    }

    #[derive(Serialize)]
    struct Enum<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        first: Option<&'a String>,
    }

    #[derive(Serialize)]
    struct Macro<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Function<'a> {
        is_async: bool,
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Module<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
    }

    let doc = cx.context.meta_by_hash(hash).into_iter().find(|m| matches!(m.kind, Kind::Module)).map(|m| m.docs).unwrap_or_default();

    let mut types = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut macros = Vec::new();
    let mut functions = Vec::new();
    let mut modules = Vec::new();

    for (_, name) in cx.context.iter_components(&cx.item) {
        let item = cx.item.join([name]);

        for meta in cx.context.meta(&item) {
            match meta.kind {
                Kind::Type { .. } => {
                    queue.push_front(Build::Type(meta.item, meta.hash));
                    let path = cx.item_path(&item, ItemKind::Type)?;
                    types.push(Type {
                        item: item.clone(),
                        path,
                        name,
                        first: meta.docs.first(),
                    });
                }
                Kind::Struct { .. } => {
                    queue.push_front(Build::Struct(meta.item, meta.hash));
                    let path = cx.item_path(&item, ItemKind::Struct)?;
                    structs.push(Struct {
                        item: item.clone(),
                        path,
                        name,
                        first: meta.docs.first(),
                    });
                }
                Kind::Enum { .. } => {
                    queue.push_front(Build::Enum(meta.item, meta.hash));
                    let path = cx.item_path(&item, ItemKind::Enum)?;
                    enums.push(Enum {
                        item: item.clone(),
                        path,
                        name,
                        first: meta.docs.first(),
                    });
                }
                Kind::Macro => {
                    queue.push_front(Build::Macro(meta.item, meta.hash));

                    macros.push(Macro {
                        path: cx.item_path(meta.item, ItemKind::Macro)?,
                        item: meta.item,
                        name,
                        doc: cx.render_docs(meta.docs.get(..1).unwrap_or_default())?,
                    });
                }
                Kind::Function(f) => {
                    if matches!(f.signature, Signature::Instance { .. }) {
                        continue;
                    }

                    queue.push_front(Build::Function(meta.item, meta.hash));

                    functions.push(Function {
                        is_async: f.is_async,
                        path: cx.item_path(&item, ItemKind::Function)?,
                        item: item.clone(),
                        name,
                        args: cx.args_to_string(f.args, f.signature, f.argument_types)?,
                        doc: cx.render_docs(meta.docs.get(..1).unwrap_or_default())?,
                    });
                }
                Kind::Module => {
                    // Skip over crate items, since they are added separately.
                    if cx.item.is_empty() && meta.item.as_crate().is_some() {
                        continue;
                    }

                    queue.push_front(Build::Module(Cow::Borrowed(meta.item), meta.hash));
                    let path = cx.item_path(meta.item, ItemKind::Module)?;
                    let name = meta.item.last().context("missing name of module")?;
                    modules.push(Module { item: meta.item, name, path })
                }
                _ => {
                    continue;
                }
            }
        }
    }

    cx.write_file(|cx| {
        cx.module_template.render(&Params {
            shared: cx.shared(),
            item: &cx.item,
            module: cx.module_path_html(true)?,
            doc: cx.render_docs(doc)?,
            types,
            structs,
            enums,
            macros,
            functions,
            modules,
        })
    })
}

/// Build a macro.
#[tracing::instrument(skip_all)]
fn build_macro(cx: &Ctxt<'_, '_>) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        module: String,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        doc: Option<String>,
    }

    let meta = cx.context.meta(&cx.item);

    let meta = meta
        .iter()
        .find(|m| matches!(m.kind, Kind::Macro))
        .context("Expected a macro")?;

    let name = cx.item.last().context("Missing macro name")?;
    let doc = cx.render_docs(meta.docs)?;

    cx.write_file(|cx| {
        cx.macro_template.render(&Params {
            shared: cx.shared(),
            module: cx.module_path_html(false)?,
            item: &cx.item,
            name,
            doc,
        })
    })
}

/// Build a function.
#[tracing::instrument(skip_all)]
fn build_function(cx: &Ctxt<'_, '_>) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        module: String,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
        return_type: Option<String>,
    }

    let meta = cx.context.meta(&cx.item);

    let meta = meta
        .iter()
        .find(|m| matches!(m.kind, Kind::Function(..)))
        .context("Expected a function")?;

    let (args, signature, return_type, argument_types) = match meta.kind {
        Kind::Function(Function {
            args,
            signature: signature @ Signature::Function { .. },
            return_type,
            argument_types,
            ..
        }) => (args, signature, return_type, argument_types),
        _ => bail!("found meta, but not a function"),
    };

    let name = cx.item.last().context("Missing function name")?;
    let doc = cx.render_docs(meta.docs)?;

    let return_type = match return_type {
        Some(hash) => Some(cx.link(hash, None)?),
        None => None,
    };

    cx.write_file(|cx| {
        cx.function_template.render(&Params {
            shared: cx.shared(),
            module: cx.module_path_html(false)?,
            item: &cx.item,
            name,
            args: cx.args_to_string(args, signature, argument_types)?,
            doc,
            return_type,
        })
    })
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

/// Helper for building an item path.
fn build_item_path(name: &str, item: &Item, kind: ItemKind, path: &mut RelativePathBuf) -> Result<()> {
    if item.is_empty() {
        path.push(name);
    } else {
        for c in item.iter() {
            let string = match c {
                ComponentRef::Crate(string) => string,
                ComponentRef::Str(string) => string,
                _ => continue,
            };

            path.push(string);
        }
    }

    path.set_extension(match kind {
        ItemKind::Type => "type.html",
        ItemKind::Struct => "struct.html",
        ItemKind::Enum => "enum.html",
        ItemKind::Module => "module.html",
        ItemKind::Macro => "macro.html",
        ItemKind::Function => "fn.html",
    });

    Ok(())
}

/// Render documentation.
fn render_code_by_syntax<I>(syntax_set: &SyntaxSet, lines: I, syntax: &SyntaxReference) -> Result<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut buf = String::new();

    let mut gen = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        syntax_set,
        ClassStyle::Spaced,
    );

    for line in lines {
        let line = line.as_ref();
        let line = line.strip_prefix(' ').unwrap_or(line);

        if line.starts_with('#') {
            continue;
        }

        buf.clear();
        buf.push_str(line);
        buf.push('\n');
        gen.parse_html_for_line_which_includes_newline(&buf)?;
    }

    Ok(gen.finalize())
}

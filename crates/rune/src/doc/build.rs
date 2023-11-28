mod enum_;
mod js;
mod markdown;
mod type_;

use core::fmt;
use core::str;

use rust_alloc::string::ToString;

use anyhow::{anyhow, bail, Context as _, Result};
use relative_path::{RelativePath, RelativePathBuf};
use serde::{Serialize, Serializer};
use syntect::highlighting::ThemeSet;
use syntect::html::{self, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::try_format;
use crate::alloc::{self, String, Vec, VecDeque};
use crate::compile::{ComponentRef, Item, ItemBuf};
use crate::doc::artifacts::Test;
use crate::doc::context::{Function, Kind, Meta, Signature};
use crate::doc::templating;
use crate::doc::{Artifacts, Context, Visitor};
use crate::std;
use crate::std::borrow::ToOwned;
use crate::Hash;

// InspiredGitHub
// Solarized (dark)
// Solarized (light)
// base16-eighties.dark
// base16-mocha.dark
// base16-ocean.dark
// base16-ocean.light
const THEME: &str = "base16-eighties.dark";
const RUNEDOC_CSS: &str = "runedoc.css";

pub(crate) struct Builder<'m> {
    state: State,
    builder: rust_alloc::boxed::Box<dyn FnOnce(&Ctxt<'_, '_>) -> Result<String> + 'm>,
}

impl<'m> Builder<'m> {
    fn new<B>(cx: &Ctxt<'_, '_>, builder: B) -> alloc::Result<Self>
    where
        B: FnOnce(&Ctxt<'_, '_>) -> Result<String> + 'm,
    {
        Ok(Self {
            state: cx.state.try_clone()?,
            builder: rust_alloc::boxed::Box::new(builder),
        })
    }
}

mod embed {
    use rust_alloc::boxed::Box;
    use rust_alloc::string::String;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "src/doc/static"]
    pub(super) struct Assets;
}

/// Build documentation based on the given context and visitors.
pub(crate) fn build(
    name: &str,
    artifacts: &mut Artifacts,
    context: &crate::Context,
    visitors: &[Visitor],
) -> Result<()> {
    let context = Context::new(context, visitors);

    let paths = templating::Paths::default();

    let partials = [("layout", asset_str("layout.html.hbs")?)];

    let templating = templating::Templating::new(partials, paths.clone())?;

    let syntax_set = SyntaxSet::load_defaults_newlines();
    let theme_set = ThemeSet::load_defaults();

    let mut fonts = Vec::new();
    let mut css = Vec::new();
    let mut js = Vec::new();

    for file in embed::Assets::iter() {
        let path = RelativePath::new(file.as_ref());

        let out = match path.extension() {
            Some("woff2") => &mut fonts,
            Some("css") => &mut css,
            Some("js") => &mut js,
            _ => continue,
        };

        let file = embed::Assets::get(file.as_ref()).context("missing asset")?;
        let data = Cow::try_from(file.data)?;
        let builder_path = artifacts.asset(true, path, move || Ok(data))?;
        paths.insert(path.as_str(), builder_path.as_str())?;
        out.try_push(builder_path)?;
    }

    let theme = theme_set.themes.get(THEME).context("missing theme")?;

    let syntax_css = artifacts.asset(true, "syntax.css", || {
        let content = String::try_from(html::css_for_theme_with_class_style(
            theme,
            html::ClassStyle::Spaced,
        )?)?;
        Ok(content.into_bytes().into())
    })?;

    paths.insert("syntax.css", syntax_css.as_str())?;
    css.try_push(syntax_css)?;

    let runedoc_css = artifacts.asset(true, RUNEDOC_CSS, || {
        let runedoc = compile(&templating, "runedoc.css.hbs")?;
        let string = runedoc.render(&())?;
        Ok(string.into_bytes().into())
    })?;

    paths.insert(RUNEDOC_CSS, runedoc_css.as_str())?;
    css.try_push(runedoc_css)?;

    // Collect an ordered set of modules, so we have a baseline of what to render when.
    let mut initial = VecDeque::new();

    for item in context.iter_modules() {
        let item = item?;
        let meta = context
            .meta(&item)?
            .into_iter()
            .find(|m| matches!(&m.kind, Kind::Module))
            .with_context(|| anyhow!("Missing meta for {item}"))?;
        initial.try_push_back(Build::Module(meta))?;
    }

    let search_index = RelativePath::new("index.js");

    let mut cx = Ctxt {
        state: State::default(),
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
        tests: Vec::new(),
    };

    let mut queue = initial.into_iter().try_collect::<VecDeque<_>>()?;

    let mut modules = Vec::new();
    let mut builders = Vec::new();

    while let Some(build) = queue.pop_front() {
        match build {
            Build::Type(meta) => {
                cx.set_path(meta)?;
                let (builder, items) = self::type_::build(&mut cx, "Type", "type", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(items)?;
            }
            Build::Struct(meta) => {
                cx.set_path(meta)?;
                let (builder, index) = self::type_::build(&mut cx, "Struct", "struct", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(index)?;
            }
            Build::Enum(meta) => {
                cx.set_path(meta)?;
                let (builder, index) = self::enum_::build(&mut cx, meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(index)?;
            }
            Build::Macro(meta) => {
                cx.set_path(meta)?;
                builders.try_push(build_macro(&mut cx, meta)?)?;
            }
            Build::Function(meta) => {
                cx.set_path(meta)?;
                builders.try_push(build_function(&mut cx, meta)?)?;
            }
            Build::Module(meta) => {
                cx.set_path(meta)?;
                builders.try_push(module(&mut cx, meta, &mut queue)?)?;
                let item = meta.item.context("Missing item")?;
                modules.try_push((item, cx.state.path.clone()))?;
            }
        }
    }

    let search_index_path = artifacts.asset(true, "index.js", || {
        let content = build_search_index(&cx)?;
        Ok(content.into_bytes().into())
    })?;

    cx.search_index = Some(&search_index_path);

    cx.state.path = RelativePath::new("index.html").to_owned();
    builders.try_push(build_index(&cx, modules)?)?;

    for builder in builders {
        cx.state = builder.state;
        artifacts.asset(false, &cx.state.path, || {
            Ok((builder.builder)(&cx)?.into_bytes().try_into()?)
        })?;
    }

    artifacts.set_tests(cx.tests);
    Ok(())
}

fn build_search_index(cx: &Ctxt) -> Result<String> {
    let mut s = String::new();
    write!(s, "window.INDEX = [")?;
    let mut it = cx.index.iter();

    while let Some(IndexEntry {
        path,
        item,
        kind,
        doc,
    }) = it.next()
    {
        write!(s, "[\"{path}\",\"{item}\",\"{kind}\",\"")?;

        if let Some(doc) = doc {
            js::encode_quoted(&mut s, doc)?;
        }

        write!(s, "\"]")?;

        if it.clone().next().is_some() {
            write!(s, ",")?;
        }
    }

    write!(s, "];")?;
    writeln!(s)?;
    Ok(s)
}

#[derive(Serialize)]
struct Shared<'a> {
    data_path: Option<&'a RelativePath>,
    search_index: Option<RelativePathBuf>,
    fonts: Vec<RelativePathBuf>,
    css: Vec<RelativePathBuf>,
    js: Vec<RelativePathBuf>,
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) enum ItemKind {
    Type,
    Struct,
    Enum,
    #[default]
    Module,
    Macro,
    Function,
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
    Variant,
}

impl fmt::Display for IndexKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexKind::Item(item) => item.fmt(f),
            IndexKind::Method => "method".fmt(f),
            IndexKind::Variant => "variant".fmt(f),
        }
    }
}

pub(crate) struct IndexEntry<'m> {
    pub(crate) path: RelativePathBuf,
    pub(crate) item: Cow<'m, Item>,
    pub(crate) kind: IndexKind,
    pub(crate) doc: Option<String>,
}

#[derive(Default, TryClone)]
pub(crate) struct State {
    #[try_clone(with = RelativePathBuf::clone)]
    path: RelativePathBuf,
    item: ItemBuf,
}

pub(crate) struct Ctxt<'a, 'm> {
    state: State,
    /// A collection of all items visited.
    index: Vec<IndexEntry<'m>>,
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
    tests: Vec<Test>,
}

impl<'m> Ctxt<'_, 'm> {
    fn set_path(&mut self, meta: Meta<'m>) -> Result<()> {
        let item_kind = match &meta.kind {
            Kind::Type => ItemKind::Type,
            Kind::Struct => ItemKind::Struct,
            Kind::Enum => ItemKind::Enum,
            Kind::Macro => ItemKind::Macro,
            Kind::Function(..) => ItemKind::Function,
            Kind::Module => ItemKind::Module,
            kind => bail!("Cannot set path for {kind:?}"),
        };

        let item = meta.item.context("Missing meta item")?;

        self.state.path = RelativePathBuf::new();
        self.state.item = item.try_to_owned()?;

        build_item_path(self.name, item, item_kind, &mut self.state.path)?;

        let doc = self.render_line_docs(meta, meta.docs.get(..1).unwrap_or_default())?;

        self.index.try_push(IndexEntry {
            path: self.state.path.clone(),
            item: Cow::Borrowed(item),
            kind: IndexKind::Item(item_kind),
            doc,
        })?;

        Ok(())
    }

    fn dir(&self) -> &RelativePath {
        self.state.path.parent().unwrap_or(RelativePath::new(""))
    }

    fn shared(&self) -> Result<Shared<'_>> {
        let dir = self.dir();

        Ok(Shared {
            data_path: self.state.path.parent(),
            search_index: self.search_index.map(|p| dir.relative(p)),
            fonts: self.fonts.iter().map(|f| dir.relative(f)).try_collect()?,
            css: self.css.iter().map(|f| dir.relative(f)).try_collect()?,
            js: self.js.iter().map(|f| dir.relative(f)).try_collect()?,
        })
    }

    /// Render rust code.
    fn render_code<I>(&self, lines: I) -> Result<String>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let syntax = match self
            .syntax_set
            .find_syntax_by_token(self::markdown::RUST_TOKEN)
        {
            Some(syntax) => syntax,
            None => self.syntax_set.find_syntax_plain_text(),
        };

        Ok(try_format!(
            "<pre><code class=\"language-rune\">{}</code></pre>",
            render_code_by_syntax(&self.syntax_set, lines, syntax, None)?
        ))
    }

    /// Render line docs.
    fn render_line_docs<S>(&mut self, meta: Meta<'_>, docs: &[S]) -> Result<Option<String>>
    where
        S: AsRef<str>,
    {
        self.render_docs(meta, docs, false)
    }

    /// Render documentation.
    fn render_docs<S>(
        &mut self,
        meta: Meta<'_>,
        docs: &[S],
        capture_tests: bool,
    ) -> Result<Option<String>>
    where
        S: AsRef<str>,
    {
        use pulldown_cmark::{BrokenLink, Options, Parser};

        if docs.is_empty() {
            return Ok(None);
        }

        let mut input = String::new();

        for line in docs {
            let line = line.as_ref();
            let line = line.strip_prefix(' ').unwrap_or(line);
            input.try_push_str(line)?;
            input.try_push('\n')?;
        }

        let mut o = String::new();
        write!(o, "<div class=\"docs\">")?;
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let mut link_error = None;

        let mut callback = |link: BrokenLink<'_>| {
            let (path, title) = match self.link_callback(meta, link.reference.as_ref()) {
                Ok(out) => out?,
                Err(error) => {
                    link_error = Some(error);
                    return None;
                }
            };

            Some((path.to_string().into(), title.into_std().into()))
        };

        let iter = Parser::new_with_broken_link_callback(&input, options, Some(&mut callback));

        let mut tests = Vec::new();

        markdown::push_html(
            &self.syntax_set,
            &mut o,
            iter,
            capture_tests.then_some(&mut tests),
        )?;

        if let Some(error) = link_error {
            return Err(error);
        }

        if capture_tests && !tests.is_empty() {
            for (content, params) in tests {
                self.tests.try_push(Test {
                    item: self.state.item.try_clone()?,
                    content,
                    params,
                })?;
            }
        }

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
    fn module_path_html(&self, meta: Meta<'_>, is_module: bool) -> Result<String> {
        let mut module = Vec::new();
        let item = meta.item.context("Missing module item")?;

        let mut iter = item.iter();

        while iter.next_back().is_some() {
            if let Some(name) = iter.as_item().last() {
                let url = self.item_path(iter.as_item(), ItemKind::Module)?;
                module.try_push(try_format!("<a class=\"module\" href=\"{url}\">{name}</a>"))?;
            }
        }

        module.reverse();

        if is_module {
            if let Some(name) = item.last() {
                module.try_push(try_format!("<span class=\"module\">{name}</span>"))?;
            }
        }

        let mut string = String::new();

        let mut it = module.into_iter();

        let last = it.next_back();

        for c in it {
            string.try_push_str(c.as_str())?;
            string.try_push_str("::")?;
        }

        if let Some(c) = last {
            string.try_push_str(c.as_str())?;
        }

        Ok(string)
    }

    /// Convert a hash into a link.
    fn link(&self, hash: Hash, text: Option<&str>) -> Result<Option<String>> {
        fn into_item_kind(meta: Meta<'_>) -> Option<ItemKind> {
            match &meta.kind {
                Kind::Type => Some(ItemKind::Type),
                Kind::Struct => Some(ItemKind::Struct),
                Kind::Enum => Some(ItemKind::Enum),
                Kind::Function { .. } => Some(ItemKind::Function),
                _ => None,
            }
        }

        let mut it = self
            .context
            .meta_by_hash(hash)?
            .into_iter()
            .flat_map(|m| Some((m, into_item_kind(m)?)));

        let Some((meta, kind)) = it.next() else {
            tracing::warn!(?hash, "No link for hash");

            for meta in self.context.meta_by_hash(hash)? {
                tracing::warn!("Candidate: {:?}", meta.kind);
            }

            return Ok(Some(try_format!("{hash}")));
        };

        let item = meta.item.context("Missing item link meta")?;

        let name = match text {
            Some(text) => text,
            None => item
                .last()
                .and_then(|c| c.as_str())
                .context("missing name")?,
        };

        let path = self.item_path(item, kind)?;
        Ok(Some(try_format!(
            "<a class=\"{kind}\" href=\"{path}\">{name}</a>"
        )))
    }

    /// Coerce args into string.
    fn args_to_string(
        &self,
        arg_names: Option<&[String]>,
        args: Option<usize>,
        sig: Signature,
        argument_types: &[Option<Hash>],
    ) -> Result<String> {
        use std::borrow::Cow;

        let mut string = String::new();
        let mut types = argument_types.iter();

        let mut args_iter;
        let mut function_iter;
        let mut instance_iter;

        let it: &mut dyn Iterator<Item = Cow<'_, str>> = if let Some(arg_names) = arg_names {
            args_iter = arg_names.iter().map(|s| Cow::Borrowed(s.as_str()));
            &mut args_iter
        } else {
            match sig {
                Signature::Function => {
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
                Signature::Instance => {
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
                    if let Some(link) = self.link(*hash, Some("self"))? {
                        string.try_push_str(&link)?;
                    } else {
                        string.try_push_str("self")?;
                    }
                } else {
                    string.try_push_str("self")?;
                }
            } else {
                string.try_push_str(arg.as_ref())?;

                if let Some(Some(hash)) = types.next() {
                    if let Some(link) = self.link(*hash, None)? {
                        string.try_push_str(": ")?;
                        string.try_push_str(&link)?;
                    }
                }
            }

            if it.peek().is_some() {
                write!(string, ", ")?;
            }
        }

        Ok(string)
    }

    fn link_callback(
        &self,
        meta: Meta<'_>,
        link: &str,
    ) -> Result<Option<(RelativePathBuf, String)>> {
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

        let Some(item) = meta.item else {
            return Ok(None);
        };

        let item = if matches!(meta.kind, Kind::Module) {
            item.join([link])?
        } else {
            let Some(parent) = item.parent() else {
                return Ok(None);
            };

            parent.join([link])?
        };

        let item_path = 'out: {
            let mut alts = Vec::new();

            for meta in self.context.meta(&item)? {
                alts.try_push(match meta.kind {
                    Kind::Struct if flavor.is_struct() => ItemKind::Struct,
                    Kind::Enum if flavor.is_enum() => ItemKind::Enum,
                    Kind::Macro if flavor.is_macro() => ItemKind::Macro,
                    Kind::Function(_) if flavor.is_function() => ItemKind::Function,
                    _ => {
                        continue;
                    }
                })?;
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

            return Ok(None);
        };

        let path = self.item_path(&item, item_path)?;
        let title = try_format!("{item_path} {link}");
        Ok(Some((path, title)))
    }
}

enum Build<'a> {
    Type(Meta<'a>),
    Struct(Meta<'a>),
    Enum(Meta<'a>),
    Macro(Meta<'a>),
    Function(Meta<'a>),
    Module(Meta<'a>),
}

/// Get an asset as a string.
fn asset_str(path: &str) -> Result<Cow<'static, str>> {
    let asset = embed::Assets::get(path).with_context(|| anyhow!("{path}: missing asset"))?;

    let data = match asset.data {
        rust_alloc::borrow::Cow::Borrowed(data) => {
            Cow::Borrowed(str::from_utf8(data).with_context(|| anyhow!("{path}: not utf-8"))?)
        }
        rust_alloc::borrow::Cow::Owned(data) => Cow::Owned(
            String::from_utf8(data.try_into()?).with_context(|| anyhow!("{path}: not utf-8"))?,
        ),
    };

    Ok(data)
}

/// Compile a template.
fn compile(templating: &templating::Templating, path: &str) -> Result<templating::Template> {
    let template = asset_str(path)?;
    templating.compile(template.as_ref())
}

#[tracing::instrument(skip_all)]
fn build_index<'m>(
    cx: &Ctxt<'_, 'm>,
    mods: Vec<(&'m Item, RelativePathBuf)>,
) -> Result<Builder<'m>> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        modules: Vec<Module<'a>>,
    }

    #[derive(Serialize)]
    struct Module<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        path: RelativePathBuf,
    }

    let mut modules = Vec::new();

    for (item, path) in mods {
        let mut c = item.iter();

        match c.next() {
            None => {}
            Some(ComponentRef::Crate(..)) => {}
            _ => continue,
        }

        if c.next().is_some() {
            continue;
        }

        modules.try_push(Module { item, path })?;
    }

    Ok(Builder::new(cx, move |cx| {
        cx.index_template.render(&Params {
            shared: cx.shared()?,
            modules,
        })
    })?)
}

/// Build a single module.
#[tracing::instrument(skip_all)]
fn module<'m>(
    cx: &mut Ctxt<'_, 'm>,
    meta: Meta<'m>,
    queue: &mut VecDeque<Build<'m>>,
) -> Result<Builder<'m>> {
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
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Struct<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Enum<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: ItemBuf,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        doc: Option<String>,
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
        deprecated: Option<&'a str>,
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
        doc: Option<String>,
    }

    let meta_item = meta.item.context("Missing item")?;

    let mut types = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut macros = Vec::new();
    let mut functions = Vec::new();
    let mut modules = Vec::new();

    for (_, name) in cx.context.iter_components(meta_item)? {
        let item = meta_item.join([name])?;

        for m in cx.context.meta(&item)? {
            match m.kind {
                Kind::Type { .. } => {
                    queue.try_push_front(Build::Type(m))?;
                    let path = cx.item_path(&item, ItemKind::Type)?;
                    types.try_push(Type {
                        item: item.try_clone()?,
                        path,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Struct { .. } => {
                    queue.try_push_front(Build::Struct(m))?;
                    let path = cx.item_path(&item, ItemKind::Struct)?;
                    structs.try_push(Struct {
                        item: item.try_clone()?,
                        path,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Enum { .. } => {
                    queue.try_push_front(Build::Enum(m))?;
                    let path = cx.item_path(&item, ItemKind::Enum)?;
                    enums.try_push(Enum {
                        item: item.try_clone()?,
                        path,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Macro => {
                    let item = m.item.context("Missing macro item")?;

                    queue.try_push_front(Build::Macro(m))?;

                    macros.try_push(Macro {
                        path: cx.item_path(item, ItemKind::Macro)?,
                        item,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Function(f) => {
                    if matches!(f.signature, Signature::Instance { .. }) {
                        continue;
                    }

                    queue.try_push_front(Build::Function(m))?;

                    functions.try_push(Function {
                        is_async: f.is_async,
                        deprecated: meta.deprecated,
                        path: cx.item_path(&item, ItemKind::Function)?,
                        item: item.try_clone()?,
                        name,
                        args: cx.args_to_string(
                            f.arg_names,
                            f.args,
                            f.signature,
                            f.argument_types,
                        )?,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Module => {
                    let item = m.item.context("Missing module item")?;

                    // Skip over crate items, since they are added separately.
                    if meta_item.is_empty() && item.as_crate().is_some() {
                        continue;
                    }

                    queue.try_push_front(Build::Module(m))?;
                    let path = cx.item_path(item, ItemKind::Module)?;
                    let name = item.last().context("missing name of module")?;
                    modules.try_push(Module {
                        item,
                        name,
                        path,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                _ => {
                    continue;
                }
            }
        }
    }

    let doc = cx.render_docs(meta, meta.docs, true)?;

    Ok(Builder::new(cx, move |cx| {
        cx.module_template.render(&Params {
            shared: cx.shared()?,
            item: meta_item,
            module: cx.module_path_html(meta, true)?,
            doc,
            types,
            structs,
            enums,
            macros,
            functions,
            modules,
        })
    })?)
}

/// Build a macro.
#[tracing::instrument(skip_all)]
fn build_macro<'m>(cx: &mut Ctxt<'_, 'm>, meta: Meta<'m>) -> Result<Builder<'m>> {
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

    let doc = cx.render_docs(meta, meta.docs, true)?;
    let item = meta.item.context("Missing item")?;
    let name = item.last().context("Missing macro name")?;

    Ok(Builder::new(cx, move |cx| {
        cx.macro_template.render(&Params {
            shared: cx.shared()?,
            module: cx.module_path_html(meta, false)?,
            item,
            name,
            doc,
        })
    })?)
}

/// Build a function.
#[tracing::instrument(skip_all)]
fn build_function<'m>(cx: &mut Ctxt<'_, 'm>, meta: Meta<'m>) -> Result<Builder<'m>> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared<'a>,
        module: String,
        is_async: bool,
        deprecated: Option<&'a str>,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
        return_type: Option<String>,
    }

    let f = match meta.kind {
        Kind::Function(
            f @ Function {
                signature: Signature::Function { .. },
                ..
            },
        ) => f,
        _ => bail!("found meta, but not a function"),
    };

    let doc = cx.render_docs(meta, meta.docs, true)?;

    let return_type = match f.return_type {
        Some(hash) => cx.link(hash, None)?,
        None => None,
    };

    let item = meta.item.context("Missing item")?;
    let deprecated = meta.deprecated;
    let name = item.last().context("Missing item name")?;

    Ok(Builder::new(cx, move |cx| {
        cx.function_template.render(&Params {
            shared: cx.shared()?,
            module: cx.module_path_html(meta, false)?,
            is_async: f.is_async,
            deprecated,
            item,
            name,
            args: cx.args_to_string(f.arg_names, f.args, f.signature, f.argument_types)?,
            doc,
            return_type,
        })
    })?)
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

/// Helper for building an item path.
fn build_item_path(
    name: &str,
    item: &Item,
    kind: ItemKind,
    path: &mut RelativePathBuf,
) -> Result<()> {
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
fn render_code_by_syntax<I>(
    syntax_set: &SyntaxSet,
    lines: I,
    syntax: &SyntaxReference,
    mut out: Option<&mut String>,
) -> Result<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut buf = String::new();

    let mut gen =
        ClassedHTMLGenerator::new_with_class_style(syntax, syntax_set, ClassStyle::Spaced);

    for line in lines {
        let line = line.as_ref();
        let line = line.strip_prefix(' ').unwrap_or(line);

        if line.starts_with('#') {
            if let Some(o) = out.as_mut() {
                o.try_push_str(line.trim_start_matches('#'))?;
                o.try_push('\n')?;
            }

            continue;
        }

        if let Some(o) = out.as_mut() {
            o.try_push_str(line)?;
            o.try_push('\n')?;
        }

        buf.clear();
        buf.try_push_str(line)?;
        buf.try_push('\n')?;
        gen.parse_html_for_line_which_includes_newline(&buf)?;
    }

    Ok(gen.finalize().try_into()?)
}

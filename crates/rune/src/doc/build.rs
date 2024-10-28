mod js;
mod type_;

use core::fmt;
use core::str;

use rust_alloc::string::ToString;

use anyhow::{anyhow, bail, Context as _, Result};
use relative_path::{RelativePath, RelativePathBuf};
use serde::{Serialize, Serializer};
use syntect::highlighting::ThemeSet;
use syntect::html;
use syntect::parsing::SyntaxSet;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashSet, VecDeque};
use crate::compile::meta;
use crate::doc::artifacts::{Test, TestKind};
use crate::doc::context::{Function, Kind, Meta, Signature};
use crate::doc::templating;
use crate::doc::{Artifacts, Context, Visitor};
use crate::item::ComponentRef;
use crate::runtime::static_type;
use crate::std::borrow::ToOwned;
use crate::{Hash, Item};

use super::markdown;

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
    state: State<'m>,
    builder: rust_alloc::boxed::Box<dyn FnOnce(&Ctxt<'_, '_>) -> Result<String> + 'm>,
}

impl<'m> Builder<'m> {
    fn new<B>(cx: &Ctxt<'_, 'm>, builder: B) -> alloc::Result<Self>
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
    #[cfg(debug_assertions)]
    use rust_alloc::boxed::Box;
    #[cfg(debug_assertions)]
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
    context: Option<&crate::Context>,
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
    let mut initial = Vec::new();
    let mut initial_seen = HashSet::new();

    for item in context.iter_modules() {
        let item = item?;

        let meta = context
            .meta(&item)?
            .into_iter()
            .find(|m| matches!(&m.kind, Kind::Module))
            .with_context(|| anyhow!("Missing meta for {item}"))?;

        if !initial_seen.try_insert(meta.hash)? {
            continue;
        }

        initial.try_push((Build::Module, meta))?;
    }

    initial.sort_by_key(|(_, meta)| meta.item);

    let search_index = RelativePath::new("index.js");
    let root_index = RelativePath::new("index.html");

    let mut cx = Ctxt {
        state: State::default(),
        index: Vec::new(),
        name,
        context: &context,
        search_index: Some(search_index),
        root_index,
        fonts: &fonts,
        css: &css,
        js: &js,
        index_template: compile(&templating, "index.html.hbs")?,
        module_template: compile(&templating, "module.html.hbs")?,
        type_template: compile(&templating, "type.html.hbs")?,
        macro_template: compile(&templating, "macro.html.hbs")?,
        function_template: compile(&templating, "function.html.hbs")?,
        syntax_set,
        tests: Vec::new(),
    };

    let mut queue = initial.into_iter().try_collect::<VecDeque<_>>()?;

    let mut modules = Vec::new();
    let mut builders = Vec::new();
    let mut visited = HashSet::new();

    while let Some((build, meta)) = queue.pop_front() {
        if !visited.try_insert((build, meta.hash))? {
            tracing::error!(?build, ?meta.item, "Already visited");
            continue;
        }

        cx.set_path(meta)?;

        tracing::trace!(?build, ?meta.item, ?cx.state.path, "Building");

        match build {
            Build::Type => {
                let (builder, items) = self::type_::build(&mut cx, "Type", "type", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(items)?;
            }
            Build::Trait => {
                let (builder, items) = self::type_::build(&mut cx, "Trait", "trait", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(items)?;
            }
            Build::Struct => {
                let (builder, index) = self::type_::build(&mut cx, "Struct", "struct", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(index)?;
            }
            Build::Enum => {
                let (builder, index) = self::type_::build(&mut cx, "Enum", "enum", meta)?;
                builders.try_push(builder)?;
                cx.index.try_extend(index)?;
            }
            Build::Macro => {
                builders.try_push(build_macro(&mut cx, meta)?)?;
            }
            Build::Function => {
                builders.try_push(build_function(&mut cx, meta)?)?;
            }
            Build::Module => {
                builders.try_push(module(&mut cx, meta, &mut queue)?)?;
                modules.try_push((meta.item, cx.state.path.clone()))?;
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
            Ok((builder.builder)(&cx)?.into_bytes().into())
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
    root_index: RelativePathBuf,
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
    Trait,
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
            ItemKind::Trait => "trait".fmt(f),
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
pub(crate) struct State<'m> {
    #[try_clone(with = RelativePathBuf::clone)]
    path: RelativePathBuf,
    #[try_clone(copy)]
    item: &'m Item,
    #[try_clone(copy)]
    kind: TestKind,
}

pub(crate) struct Ctxt<'a, 'm> {
    state: State<'m>,
    /// A collection of all items visited.
    index: Vec<IndexEntry<'m>>,
    name: &'a str,
    context: &'a Context<'m>,
    search_index: Option<&'a RelativePath>,
    root_index: &'a RelativePath,
    fonts: &'a [RelativePathBuf],
    css: &'a [RelativePathBuf],
    js: &'a [RelativePathBuf],
    index_template: templating::Template,
    module_template: templating::Template,
    type_template: templating::Template,
    macro_template: templating::Template,
    function_template: templating::Template,
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
            Kind::Trait => ItemKind::Trait,
            kind => bail!("Cannot set path for {kind:?}"),
        };

        self.state.kind = TestKind::default();
        self.state.path = RelativePathBuf::new();
        self.state.item = meta.item;

        build_item_path(self.name, meta.item, item_kind, &mut self.state.path)?;

        let doc = self.render_line_docs(meta, meta.docs.get(..1).unwrap_or_default())?;

        self.index.try_push(IndexEntry {
            path: self.state.path.clone(),
            item: Cow::Borrowed(meta.item),
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
            root_index: dir.relative(self.root_index),
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
            markdown::render_code_by_syntax(&self.syntax_set, lines, syntax, None)?
        ))
    }

    /// Render an optional return type parameter.
    ///
    /// Returning `None` indicates that the return type is the default return
    /// type, which is `()`.
    fn return_type(&self, ty: &meta::DocType) -> Result<Option<String>> {
        match *ty {
            meta::DocType {
                base, ref generics, ..
            } if static_type::TUPLE == base && generics.is_empty() => Ok(None),
            meta::DocType {
                base, ref generics, ..
            } => Ok(Some(self.link(base, None, generics)?)),
        }
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

        for (content, params) in tests {
            self.tests.try_push(Test {
                item: self.state.item.try_to_owned()?,
                kind: self.state.kind,
                content,
                params,
            })?;
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

    /// Build backlinks for the current item.
    fn module_path_html(&self, meta: Meta<'_>, is_module: bool) -> Result<String> {
        fn unqualified_component<'a>(c: &'a ComponentRef<'_>) -> &'a dyn fmt::Display {
            match c {
                ComponentRef::Crate(name) => name,
                ComponentRef::Str(name) => name,
                c => c,
            }
        }

        let mut module = Vec::new();

        let mut iter = meta.item.iter();

        while iter.next_back().is_some() {
            if let Some(c) = iter.as_item().last() {
                let name: &dyn fmt::Display = unqualified_component(&c);
                let url = self.item_path(iter.as_item(), ItemKind::Module)?;
                module.try_push(try_format!("<a class=\"module\" href=\"{url}\">{name}</a>"))?;
            }
        }

        module.reverse();

        if is_module {
            if let Some(c) = meta.item.last() {
                let name: &dyn fmt::Display = unqualified_component(&c);
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
    fn link(&self, hash: Hash, text: Option<&str>, generics: &[meta::DocType]) -> Result<String> {
        let mut s = String::new();
        self.write_link(&mut s, hash, text, generics)?;
        Ok(s)
    }

    /// Write a placeholder for the `any` type.
    fn write_any(&self, o: &mut dyn TryWrite) -> Result<()> {
        write!(o, "<span class=\"any\">any</span>")?;
        Ok(())
    }

    /// Convert a hash into a link.
    fn write_link(
        &self,
        o: &mut dyn TryWrite,
        hash: Hash,
        text: Option<&str>,
        generics: &[meta::DocType],
    ) -> Result<()> {
        fn into_item_kind(meta: Meta<'_>) -> Option<ItemKind> {
            match &meta.kind {
                Kind::Type => Some(ItemKind::Type),
                Kind::Struct => Some(ItemKind::Struct),
                Kind::Enum => Some(ItemKind::Enum),
                Kind::Function { .. } => Some(ItemKind::Function),
                _ => None,
            }
        }

        let Some(hash) = hash.as_non_empty() else {
            self.write_any(o)?;
            return Ok(());
        };

        if static_type::TUPLE == hash && text.is_none() {
            write!(o, "(")?;
            self.write_generics(o, generics)?;
            write!(o, ")")?;
            return Ok(());
        }

        let mut it = self
            .context
            .meta_by_hash(hash)?
            .into_iter()
            .flat_map(|m| Some((m, into_item_kind(m)?)));

        let outcome = 'out: {
            let Some((meta, kind)) = it.next() else {
                tracing::warn!(?hash, "No link for hash");

                for _meta in self.context.meta_by_hash(hash)? {
                    tracing::warn!("Candidate: {:?}", _meta.kind);
                }

                break 'out (None, None, text);
            };

            let text = match text {
                Some(text) => Some(text),
                None => meta.item.last().and_then(|c| c.as_str()),
            };

            (Some(self.item_path(meta.item, kind)?), Some(kind), text)
        };

        let (path, kind, text) = outcome;

        let text: &dyn fmt::Display = match &text {
            Some(text) => text,
            None => &hash,
        };

        if let (Some(kind), Some(path)) = (kind, path) {
            write!(o, "<a class=\"{kind}\" href=\"{path}\">{text}</a>")?;
        } else {
            write!(o, "{text}")?;
        }

        if !generics.is_empty() {
            write!(o, "&lt;")?;
            self.write_generics(o, generics)?;
            write!(o, "&gt;")?;
        }

        Ok(())
    }

    fn write_generics(&self, o: &mut dyn TryWrite, generics: &[meta::DocType]) -> Result<()> {
        let mut it = generics.iter().peekable();

        while let Some(ty) = it.next() {
            self.write_link(o, ty.base, None, &ty.generics)?;

            if it.peek().is_some() {
                write!(o, ", ")?;
            }
        }

        Ok(())
    }

    /// Coerce args into string.
    fn args_to_string(
        &self,
        sig: Signature,
        arguments: Option<&[meta::DocArgument]>,
    ) -> Result<String> {
        let mut string = String::new();

        let Some(arguments) = arguments else {
            match sig {
                Signature::Function => {
                    let mut string = String::new();
                    write!(string, "..")?;
                    return Ok(string);
                }
                Signature::Instance => {
                    let mut string = String::new();
                    write!(string, "self, ..")?;
                    return Ok(string);
                }
            }
        };

        let mut it = arguments.iter().peekable();

        while let Some(arg) = it.next() {
            if matches!(sig, Signature::Instance) && arg.name.is_self() {
                if let Some(hash) = arg.base.as_non_empty() {
                    self.write_link(&mut string, hash, Some("self"), &[])?;
                } else {
                    write!(string, "self")?;
                }
            } else {
                write!(string, "{}", arg.name)?;
                string.try_push_str(": ")?;
                self.write_link(&mut string, arg.base, None, &arg.generics)?;
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

        let item = if matches!(meta.kind, Kind::Module) {
            meta.item.join([link])?
        } else {
            let Some(parent) = meta.item.parent() else {
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
                _items => {
                    tracing::warn!(?link, items = ?_items, "Bad link, got multiple items");
                }
            }

            return Ok(None);
        };

        let path = self.item_path(&item, item_path)?;
        let title = try_format!("{item_path} {link}");
        Ok(Some((path, title)))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum Build {
    Type,
    Struct,
    Enum,
    Macro,
    Function,
    Module,
    Trait,
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

    // sort the modules by name
    modules.sort_by_key(|module| module.item.as_crate().unwrap_or(""));

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
    queue: &mut VecDeque<(Build, Meta<'m>)>,
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
        traits: Vec<Trait<'a>>,
    }

    #[derive(Serialize)]
    struct Type<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Struct<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        doc: Option<String>,
    }

    #[derive(Serialize)]
    struct Enum<'a> {
        path: RelativePathBuf,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
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
        item: &'a Item,
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

    #[derive(Serialize)]
    struct Trait<'a> {
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        path: RelativePathBuf,
        doc: Option<String>,
    }

    let mut types = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut macros = Vec::new();
    let mut functions = Vec::new();
    let mut modules = Vec::new();
    let mut traits = Vec::new();

    for (_, name) in cx.context.iter_components(meta.item)? {
        let item = meta.item.join([name])?;
        tracing::trace!(?item, "Looking up");

        for m in cx.context.meta(&item)? {
            tracing::trace!(?item, ?m.kind, "Found");

            match m.kind {
                Kind::Type { .. } => {
                    queue.try_push_front((Build::Type, m))?;

                    types.try_push(Type {
                        path: cx.item_path(m.item, ItemKind::Type)?,
                        item: m.item,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Struct { .. } => {
                    queue.try_push_front((Build::Struct, m))?;

                    structs.try_push(Struct {
                        path: cx.item_path(m.item, ItemKind::Struct)?,
                        item: m.item,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Enum { .. } => {
                    queue.try_push_front((Build::Enum, m))?;

                    enums.try_push(Enum {
                        path: cx.item_path(m.item, ItemKind::Enum)?,
                        item: m.item,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Macro => {
                    queue.try_push_front((Build::Macro, m))?;

                    macros.try_push(Macro {
                        path: cx.item_path(m.item, ItemKind::Macro)?,
                        item: m.item,
                        name,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Function(f) => {
                    if matches!(f.signature, Signature::Instance { .. }) {
                        continue;
                    }

                    queue.try_push_front((Build::Function, m))?;

                    functions.try_push(Function {
                        is_async: f.is_async,
                        deprecated: meta.deprecated,
                        path: cx.item_path(m.item, ItemKind::Function)?,
                        item: m.item,
                        name,
                        args: cx.args_to_string(f.signature, f.arguments)?,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Module => {
                    // Skip over crate items, since they are added separately.
                    if meta.item.is_empty() && m.item.as_crate().is_some() {
                        continue;
                    }

                    queue.try_push_front((Build::Module, m))?;

                    let path = cx.item_path(m.item, ItemKind::Module)?;
                    let name = m.item.last().context("missing name of module")?;

                    // Prevent multiple entries of a module, with no documentation
                    modules.retain(|module: &Module<'_>| {
                        !(module.name == name && module.doc.is_none())
                    });

                    modules.try_push(Module {
                        item: m.item,
                        name,
                        path,
                        doc: cx.render_line_docs(m, m.docs.get(..1).unwrap_or_default())?,
                    })?;
                }
                Kind::Trait { .. } => {
                    queue.try_push_front((Build::Trait, m))?;

                    traits.try_push(Trait {
                        path: cx.item_path(m.item, ItemKind::Trait)?,
                        item: m.item,
                        name,
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
            item: meta.item,
            module: cx.module_path_html(meta, true)?,
            doc,
            types,
            structs,
            enums,
            macros,
            functions,
            modules,
            traits,
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
    let name = meta.item.last().context("Missing macro name")?;

    Ok(Builder::new(cx, move |cx| {
        cx.macro_template.render(&Params {
            shared: cx.shared()?,
            module: cx.module_path_html(meta, false)?,
            item: meta.item,
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
        is_test: bool,
        is_bench: bool,
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

    let return_type = cx.return_type(f.return_type)?;

    let name = meta.item.last().context("Missing item name")?;

    Ok(Builder::new(cx, move |cx| {
        cx.function_template.render(&Params {
            shared: cx.shared()?,
            module: cx.module_path_html(meta, false)?,
            is_async: f.is_async,
            is_test: f.is_test,
            is_bench: f.is_bench,
            deprecated: meta.deprecated,
            item: meta.item,
            name,
            args: cx.args_to_string(f.signature, f.arguments)?,
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
    serializer.collect_str(&item.unqalified())
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
        ItemKind::Trait => "trait.html",
    });

    Ok(())
}

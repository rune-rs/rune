mod enum_;
mod type_;

use std::fs;
use std::io;
use std::path::Path;

use crate::no_std::prelude::*;

use anyhow::{anyhow, bail, Context as _, Error, Result};
use relative_path::{RelativePath, RelativePathBuf};
use rust_embed::EmbeddedFile;
use rust_embed::RustEmbed;
use serde::{Serialize, Serializer};
use syntect::highlighting::ThemeSet;
use syntect::html::{self, ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::compile::{ComponentRef, Item, ItemBuf};
use crate::doc::context::{Function, Kind, Signature};
use crate::doc::templating;
use crate::doc::{Context, Visitor};
use crate::Hash;

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

enum ItemPath {
    Type,
    Struct,
    Enum,
    Module,
    Function,
}

fn build_item_path(name: &str, item: &Item, kind: ItemPath, path: &mut RelativePathBuf) {
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
        ItemPath::Type => "type.html",
        ItemPath::Struct => "struct.html",
        ItemPath::Enum => "enum.html",
        ItemPath::Module => "module.html",
        ItemPath::Function => "fn.html",
    });
}

struct Ctxt<'a> {
    root: &'a Path,
    item: ItemBuf,
    path: RelativePathBuf,
    name: &'a str,
    context: &'a Context<'a>,
    fonts: &'a [RelativePathBuf],
    css: &'a [RelativePathBuf],
    index_template: templating::Template,
    module_template: templating::Template,
    type_template: templating::Template,
    function_template: templating::Template,
    enum_template: templating::Template,
    syntax_set: SyntaxSet,
}

impl Ctxt<'_> {
    fn set_path(&mut self, item: ItemBuf, kind: ItemPath) {
        self.path = RelativePathBuf::new();
        build_item_path(self.name, &item, kind, &mut self.path);
        self.item = item;
    }

    fn dir(&self) -> &RelativePath {
        self.path.parent().unwrap_or(RelativePath::new(""))
    }

    fn shared(&self) -> Shared {
        let dir = self.dir();

        Shared {
            fonts: self.fonts.iter().map(|f| dir.relative(f)).collect(),
            css: self.css.iter().map(|f| dir.relative(f)).collect(),
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
    fn render_code<I>(&self, lines: I) -> String
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let syntax = match self.syntax_set.find_syntax_by_token(RUST_TOKEN) {
            Some(syntax) => syntax,
            None => self.syntax_set.find_syntax_plain_text(),
        };

        format!(
            "<pre><code class=\"language-rune\">{}</code></pre>",
            self.render_code_by_syntax(lines, syntax)
        )
    }

    /// Render documentation.
    fn render_code_by_syntax<I>(&self, lines: I, syntax: &SyntaxReference) -> String
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut buf = String::new();

        let mut gen = ClassedHTMLGenerator::new_with_class_style(
            syntax,
            &self.syntax_set,
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
            let _ = gen.parse_html_for_line_which_includes_newline(&buf);
        }

        gen.finalize()
    }

    /// Render documentation.
    fn render_docs<S>(&self, docs: &[S]) -> Result<Option<String>>
    where
        S: AsRef<str>,
    {
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
                            let html = self.cx.render_code_by_syntax(text.lines(), syntax);
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

        for line in docs {
            let line = line.as_ref();
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

    #[inline]
    fn item_path(&self, item: &Item, kind: ItemPath) -> RelativePathBuf {
        let mut path = RelativePathBuf::new();
        build_item_path(self.name, item, kind, &mut path);
        path
    }

    /// Build banklinks for the current item.
    fn module_path_html(&self, is_module: bool) -> String {
        let mut module = Vec::new();
        let mut iter = self.item.iter();
        let dir = self.dir();

        while iter.next_back().is_some() {
            if let Some(name) = iter.as_item().last() {
                let url = dir.relative(self.item_path(iter.as_item(), ItemPath::Module));
                module.push(format!("<a class=\"module\" href=\"{url}\">{name}</a>"));
            }
        }

        module.reverse();

        if is_module {
            if let Some(name) = self.item.last() {
                module.push(format!("<span class=\"module\">{name}</span>"));
            }
        }

        module.join("::")
    }

    /// Convert a hash into a link.
    fn link(&self, hash: Hash, text: Option<&str>) -> Result<String> {
        let link = if let Some(meta) = self.context.meta_by_hash(hash) {
            let name = match text {
                Some(text) => text,
                None => meta
                    .item
                    .last()
                    .and_then(|c| c.as_str())
                    .context("missing name")?,
            };

            match &meta.kind {
                Kind::Unknown => {
                    let path = self
                        .dir()
                        .relative(self.item_path(meta.item, ItemPath::Type));
                    format!("<a class=\"type\" href=\"{path}\">{name}</a>")
                }
                Kind::Struct => {
                    let path = self
                        .dir()
                        .relative(self.item_path(meta.item, ItemPath::Struct));
                    format!("<a class=\"struct\" href=\"{path}\">{name}</a>")
                }
                Kind::Enum => {
                    let path = self
                        .dir()
                        .relative(self.item_path(meta.item, ItemPath::Enum));
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
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Build<'a> {
    Type(ItemBuf, Hash),
    Struct(ItemBuf, Hash),
    Enum(ItemBuf, Hash),
    Function(ItemBuf),
    Module(&'a Item),
}

/// Compile a template.
fn compile(templating: &templating::Templating, path: &str) -> Result<templating::Template> {
    let template = Assets::get(path).with_context(|| anyhow!("{path}: missing"))?;
    let template = std::str::from_utf8(template.data.as_ref())
        .with_context(|| anyhow!("{path}: not utf-8"))?;
    templating.compile(template)
}

/// Write html documentation to the given path.
pub fn write_html(
    name: &str,
    root: &Path,
    context: &crate::Context,
    visitors: &[Visitor],
) -> Result<()> {
    let context = Context::new(context, visitors);

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
                let path = copy_file(name, root, file)?;
                fonts.push(path.to_owned());
            }
            (Some(name), Some("css")) => {
                let file = Assets::get(file.as_ref()).context("missing font")?;
                let path = copy_file(name, root, file)?;
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

    // Collect an ordered set of modules, so we have a baseline of what to render when.
    let mut initial = BTreeSet::new();
    let mut children = HashMap::<ItemBuf, BTreeSet<_>>::new();

    for module in context.iter_modules() {
        initial.insert(Build::Module(module));

        if let Some(parent) = module.parent() {
            children
                .entry(parent.to_owned())
                .or_default()
                .insert(module);
        }
    }

    let mut cx = Ctxt {
        root,
        item: ItemBuf::new(),
        path: RelativePathBuf::new(),
        name,
        context: &context,
        fonts: &fonts,
        css: &css,
        index_template: compile(&templating, "index.html.hbs")?,
        module_template: compile(&templating, "module.html.hbs")?,
        type_template: compile(&templating, "type.html.hbs")?,
        function_template: compile(&templating, "function.html.hbs")?,
        enum_template: compile(&templating, "enum.html.hbs")?,
        syntax_set,
    };

    let mut queue = initial.into_iter().collect::<VecDeque<_>>();

    let mut modules = Vec::new();

    while let Some(build) = queue.pop_front() {
        match build {
            Build::Type(item, hash) => {
                cx.set_path(item, ItemPath::Type);
                self::type_::build(&cx, "Type", "type", hash)?;
            }
            Build::Struct(item, hash) => {
                cx.set_path(item, ItemPath::Struct);
                self::type_::build(&cx, "Struct", "struct", hash)?;
            }
            Build::Enum(item, hash) => {
                cx.set_path(item, ItemPath::Enum);
                self::enum_::build(&cx, hash)?;
            }
            Build::Function(item) => {
                cx.set_path(item, ItemPath::Function);
                function(&cx)?;
            }
            Build::Module(item) => {
                cx.set_path(item.to_owned(), ItemPath::Module);
                module(&cx, &mut queue, &children)?;
                modules.push((item, cx.path.clone()));
            }
        }
    }

    cx.path = RelativePath::new("index.html").to_owned();
    index(&cx, &modules)?;
    Ok(())
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
fn index(cx: &Ctxt<'_>, mods: &[(&Item, RelativePathBuf)]) -> Result<()> {
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

    let mut modules = Vec::new();

    for (item, path) in mods {
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
fn module(
    cx: &Ctxt<'_>,
    queue: &mut VecDeque<Build>,
    children: &HashMap<ItemBuf, BTreeSet<&Item>>,
) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        module: String,
        types: Vec<Type<'a>>,
        structs: Vec<Struct<'a>>,
        enums: Vec<Enum<'a>>,
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

    let module = cx.module_path_html(true);
    let mut types = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut functions = Vec::new();
    let mut modules = Vec::new();

    for name in cx.context.iter_components(&cx.item) {
        let item = cx.item.join([name]);

        let meta = match cx.context.meta(&item) {
            Some(meta) => meta,
            _ => continue,
        };

        match meta.kind {
            Kind::Unknown { .. } => {
                queue.push_front(Build::Type(item.clone(), meta.hash));
                let path = cx.dir().relative(cx.item_path(&item, ItemPath::Type));
                types.push(Type {
                    item,
                    path,
                    name,
                    first: meta.docs.first(),
                });
            }
            Kind::Struct { .. } => {
                queue.push_front(Build::Struct(item.clone(), meta.hash));
                let path = cx.dir().relative(cx.item_path(&item, ItemPath::Struct));
                structs.push(Struct {
                    item,
                    path,
                    name,
                    first: meta.docs.first(),
                });
            }
            Kind::Enum { .. } => {
                queue.push_front(Build::Enum(item.clone(), meta.hash));
                let path = cx.dir().relative(cx.item_path(&item, ItemPath::Enum));
                enums.push(Enum {
                    item,
                    path,
                    name,
                    first: meta.docs.first(),
                });
            }
            Kind::Function(f) => {
                if !matches!(f.signature, Signature::Instance { .. }) {
                    queue.push_front(Build::Function(item.clone()));

                    functions.push(Function {
                        is_async: f.is_async,
                        path: cx.dir().relative(cx.item_path(&item, ItemPath::Function)),
                        item,
                        name,
                        args: cx.args_to_string(f.args, f.signature, f.argument_types)?,
                        doc: cx.render_docs(meta.docs.get(..1).unwrap_or_default())?,
                    });
                }
            }
            _ => {
                continue;
            }
        }
    }

    for item in children.get(&cx.item).into_iter().flatten() {
        let path = cx.dir().relative(cx.item_path(item, ItemPath::Module));
        let name = item.last().context("missing name of module")?;

        modules.push(Module { item, name, path })
    }

    cx.write_file(|cx| {
        cx.module_template.render(&Params {
            shared: cx.shared(),
            item: &cx.item,
            module,
            types,
            structs,
            enums,
            functions,
            modules,
        })
    })
}

/// Build a function.
#[tracing::instrument(skip_all)]
fn function(cx: &Ctxt<'_>) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: Shared,
        module: String,
        #[serde(serialize_with = "serialize_item")]
        item: &'a Item,
        #[serde(serialize_with = "serialize_component_ref")]
        name: ComponentRef<'a>,
        args: String,
        doc: Option<String>,
        return_type: Option<String>,
    }

    let meta = cx.context.meta(&cx.item).context("missing function")?;

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

    let name = cx.item.last().context("missing function name")?;
    let doc = cx.render_docs(meta.docs)?;

    let return_type = match return_type {
        Some(hash) => Some(cx.link(hash, None)?),
        None => None,
    };

    cx.write_file(|cx| {
        cx.function_template.render(&Params {
            shared: cx.shared(),
            module: cx.module_path_html(false),
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

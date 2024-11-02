use core::str;

use anyhow::{Context as _, Result};
use pulldown_cmark::{Options, Parser};
use syntect::parsing::SyntaxSet;

use crate as rune;
use crate::alloc::borrow::Cow;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{HashMap, String};
use crate::compile::Prelude;
use crate::doc::markdown;
use crate::doc::{Artifacts, Context, Function, Kind, Meta, Signature, Visitor};
use crate::{hash, ItemBuf};

pub(crate) fn build(
    artifacts: &mut Artifacts,
    context: &crate::Context,
    visitors: &[Visitor],
    extensions: bool,
) -> Result<()> {
    let context = Context::new(Some(context), visitors);

    let mut acx = AutoCompleteCtx::new(&context, extensions);

    for item in context.iter_modules() {
        let item = item?;
        acx.collect_meta(&item)?;
    }

    acx.build(artifacts)?;
    Ok(())
}

struct AutoCompleteCtx<'a> {
    ctx: &'a Context<'a>,
    extensions: bool,
    syntax_set: SyntaxSet,
    fixed: HashMap<ItemBuf, Meta<'a>>,
    instance: HashMap<ItemBuf, Meta<'a>>,
    prelude: Prelude,
}

impl<'a> AutoCompleteCtx<'a> {
    fn new(ctx: &'a Context, extensions: bool) -> AutoCompleteCtx<'a> {
        AutoCompleteCtx {
            ctx,
            extensions,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            fixed: HashMap::new(),
            instance: HashMap::new(),
            prelude: Prelude::with_default_prelude().unwrap_or_default(),
        }
    }

    fn collect_meta(&mut self, item: &ItemBuf) -> Result<()> {
        for meta in self.ctx.meta(item)?.into_iter() {
            match meta.kind {
                Kind::Type => {
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Struct => {
                    for (_, name) in self.ctx.iter_components(item)? {
                        let item = item.join([name])?;
                        self.collect_meta(&item)?;
                    }
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Variant => {
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Enum => {
                    for (_, name) in self.ctx.iter_components(item)? {
                        let item = item.join([name])?;
                        self.collect_meta(&item)?;
                    }
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Macro => {
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Function(f) => {
                    if matches!(f.signature, Signature::Instance) {
                        self.instance.try_insert(item.try_clone()?, meta)?;
                    } else {
                        self.fixed.try_insert(item.try_clone()?, meta)?;
                    }
                }
                Kind::Const(_) => {
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Module => {
                    for (_, name) in self.ctx.iter_components(item)? {
                        let item = item.join([name])?;
                        self.collect_meta(&item)?;
                    }
                    self.fixed.try_insert(item.try_clone()?, meta)?;
                }
                Kind::Unsupported | Kind::Trait => {}
            }
        }

        Ok(())
    }

    fn build(&mut self, artifacts: &mut Artifacts) -> Result<()> {
        let mut content = Vec::new();
        self.write(&mut content)?;

        artifacts.asset(false, "rune-autocomplete.js", || Ok(content.into()))?;

        Ok(())
    }

    fn doc_to_html(&self, meta: &Meta) -> Result<Option<String>> {
        let mut input = String::new();

        for line in meta.docs {
            let line = line.strip_prefix(' ').unwrap_or(line);
            input.try_push_str(line)?;
            input.try_push('\n')?;
        }

        let mut o = String::new();
        write!(o, "<div class=\"docs\">")?;
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let iter = Parser::new_ext(&input, options);

        markdown::push_html(Some(&self.syntax_set), &mut o, iter, None)?;

        write!(o, "</div>")?;
        let o = String::try_from(o.replace('`', "\\`"))?;

        Ok(Some(o))
    }

    fn get_name(&self, item: &ItemBuf) -> Result<String> {
        // shorten item name with auto prelude when available
        if let Some(name) = self.prelude.get_local(item) {
            return Ok(name.try_to_string()?);
        }

        // take default name and remove starting double points
        let mut name = item.try_to_string()?;
        if name.starts_with("::") {
            name.try_replace_range(..2, "")?;
        }

        Ok(name)
    }

    fn get_fn_ext(f: &Function) -> Result<String> {
        let mut ext = String::new();
        // automatic .await for async functions
        if f.is_async {
            ext.try_push_str(".await")?;
        }

        // automatic questionmark for result and option
        if matches!(
            f.return_type.base,
            hash!(::std::option::Option) | hash!(::std::result::Result)
        ) {
            ext.try_push_str("?")?;
        }

        Ok(ext)
    }

    fn get_fn_param(f: &Function) -> Result<String> {
        let mut param = String::try_from("(")?;

        // add arguments when no argument names are provided
        if let Some(args) = f.arguments {
            for (n, arg) in args.iter().enumerate() {
                if n > 0 {
                    param.try_push_str(", ")?;
                }

                write!(param, "{}", arg.name)?;
            }
        }

        param.try_push(')')?;
        Ok(param)
    }

    fn get_fn_ret_typ(&self, f: &Function) -> Result<String> {
        let mut param = String::new();

        if !self.extensions {
            return Ok(param);
        }

        if let Some(item) = self
            .ctx
            .meta_by_hash(f.return_type.base)
            .ok()
            .and_then(|v| v.into_iter().next())
            .and_then(|m| m.item.last())
        {
            param.try_push_str(" -> ")?;
            param.try_push_str(&item.try_to_string()?)?;
        }

        Ok(param)
    }

    fn write_hint(
        &self,
        f: &mut Vec<u8>,
        value: &str,
        meta: &str,
        score: usize,
        caption: Option<&str>,
        doc: Option<&str>,
    ) -> Result<()> {
        write!(f, r#"{{"#)?;
        write!(f, r#"value: "{value}""#)?;

        if let Some(caption) = caption {
            write!(f, r#", caption: "{caption}""#)?;
        }

        write!(f, r#", meta: "{meta}""#)?;
        write!(f, r#", score: {score}"#)?;

        if let Some(doc) = doc {
            let doc = escape(doc)?;
            write!(f, r#", docHTML: "{doc}""#)?;
        }

        write!(f, "}}")?;
        Ok(())
    }

    fn write_instances(&self, f: &mut Vec<u8>) -> Result<()> {
        write!(f, r#"var instance = ["#)?;

        for (item, meta) in self.instance.iter() {
            let Kind::Function(fnc) = meta.kind else {
                continue;
            };

            write!(f, "  ")?;

            let mut iter = item.iter().rev();
            let mut value = iter
                .next()
                .context("No function name found for instance function")?
                .try_to_string()?;
            value.try_push_str(&Self::get_fn_param(&fnc)?)?;

            let mut typ = String::new();
            typ.try_push_str(&value)?;
            typ.try_push_str(&self.get_fn_ret_typ(&fnc)?)?;
            value.try_push_str(&Self::get_fn_ext(&fnc)?)?;
            if let Some(pre) = iter.next().and_then(|t| t.try_to_string().ok()) {
                typ.try_push_str(" [")?;
                typ.try_push_str(&pre)?;
                typ.try_push(']')?;
            }

            let doc = self.doc_to_html(meta).ok().flatten();

            let info = if fnc.is_async {
                "async Instance"
            } else {
                "Instance"
            };

            self.write_hint(f, &value, info, 0, Some(&typ), doc.as_deref())?;
            writeln!(f, ",")?;
        }
        write!(f, "];")?;

        Ok(())
    }

    fn write_fixed(&self, f: &mut Vec<u8>) -> Result<()> {
        writeln!(f, r#"var fixed = ["#)?;

        for (item, meta) in self
            .fixed
            .iter()
            .filter(|(_, m)| !matches!(m.kind, Kind::Unsupported | Kind::Trait))
        {
            write!(f, "  ")?;

            match meta.kind {
                Kind::Type => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Type", 0, None, doc.as_deref())?;
                }
                Kind::Struct => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Struct", 0, None, doc.as_deref())?;
                }
                Kind::Variant => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Variant", 0, None, doc.as_deref())?;
                }
                Kind::Enum => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Enum", 0, None, doc.as_deref())?;
                }
                Kind::Macro => {
                    let mut name = self.get_name(item)?;
                    name.try_push_str("!()")?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Type", 0, None, doc.as_deref())?;
                }
                Kind::Function(fnc) => {
                    let mut value = self.get_name(item)?;
                    value.try_push_str(&Self::get_fn_param(&fnc)?)?;
                    let mut caption = value.try_clone()?;
                    caption.try_push_str(&self.get_fn_ret_typ(&fnc)?)?;
                    value.try_push_str(&Self::get_fn_ext(&fnc)?)?;
                    let doc = self.doc_to_html(meta).ok().flatten();

                    let info = if fnc.is_async {
                        "async Function"
                    } else {
                        "Function"
                    };

                    self.write_hint(f, &value, info, 0, Some(&caption), doc.as_deref())?;
                }
                Kind::Const(_) => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Const", 10, None, doc.as_deref())?;
                }
                Kind::Module => {
                    let name = self.get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Module", 9, None, doc.as_deref())?;
                }
                _ => {}
            }

            writeln!(f, ",")?;
        }

        writeln!(f, "];")?;
        Ok(())
    }

    fn write(&self, f: &mut Vec<u8>) -> Result<()> {
        let completer =
            super::embed::Assets::get("rune-completer.js").context("missing rune-completer.js")?;

        f.try_extend_from_slice(completer.data.as_ref())?;
        self.write_fixed(f)?;
        self.write_instances(f)?;
        Ok(())
    }
}

fn escape(s: &str) -> Result<Cow<'_, str>> {
    let n = 'escape: {
        for (n, c) in s.char_indices() {
            match c {
                '\"' | '\n' => break 'escape n,
                _ => {}
            }
        }

        return Ok(Cow::Borrowed(s));
    };

    let mut out = String::new();

    let (head, tail) = s.split_at(n);
    out.try_push_str(head)?;

    for c in tail.chars() {
        match c {
            '\"' => {
                out.try_push_str(r#"\""#)?;
            }
            '\n' => {
                out.try_push_str(r#"\n"#)?;
            }
            _ => {
                out.try_push(c)?;
            }
        }
    }

    Ok(Cow::Owned(out))
}

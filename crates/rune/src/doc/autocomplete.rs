use core::fmt::Display;

use crate::doc::{Artifacts, Context, Visitor};

use anyhow::{Context as _, Result};
use pulldown_cmark::{Options, Parser};
use rune_alloc::{fmt::TryWrite, HashMap, String};
use rune_core::{Hash, ItemBuf};
use syntect::parsing::SyntaxSet;

use super::{
    build::markdown,
    context::{Function, Kind, Meta},
};

use crate::alloc::prelude::*;

pub(crate) fn build(
    artifacts: &mut Artifacts,
    context: &crate::Context,
    visitors: &[Visitor],
    extensions: bool,
) -> Result<()> {
    let context = Context::new(context, visitors);

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
}

impl<'a> AutoCompleteCtx<'a> {
    fn new(ctx: &'a Context, extensions: bool) -> AutoCompleteCtx<'a> {
        AutoCompleteCtx {
            ctx,
            extensions,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            fixed: HashMap::new(),
            instance: HashMap::new(),
        }
    }

    fn collect_meta(&mut self, item: &ItemBuf) -> Result<()> {
        for meta in self.ctx.meta(item)?.into_iter() {
            match meta.kind {
                Kind::Unsupported => {}
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
                    if f.arg_names.into_iter().flatten().next().map(|s| s.as_str()) == Some("self")
                    {
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
            }
        }

        Ok(())
    }

    fn build(&mut self, artifacts: &mut Artifacts) -> Result<()> {
        let mut content = std::string::String::new();
        self.write(&mut content)?;

        artifacts.asset(false, "autocomplete.js", || {
            let string = String::try_from(content)?;
            Ok(string.into_bytes().into())
        })?;

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

        markdown::push_html(&self.syntax_set, &mut o, iter, None)?;

        write!(o, "</div>")?;
        let o = String::try_from(o.replace('`', "\\`"))?;

        Ok(Some(o))
    }

    fn get_name(item: &ItemBuf) -> Result<String> {
        let mut name = item.try_to_string()?;

        if name.starts_with("::std::io::print!()") {
            name.try_replace_range(..12, "")?;
        }
        if name.starts_with("::std::io::println!()") {
            name.try_replace_range(..12, "")?;
        }
        if name.starts_with("::std::io::dbg!()") {
            name.try_replace_range(..12, "")?;
        }
        if name.starts_with("::std::vec::") {
            name.try_replace_range(..12, "")?;
        }
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
        if f.return_type == Some(Hash::new(0x1978eae6b50a98ef))
            || f.return_type == Some(Hash::new(0xc0958f246e193e78))
        {
            ext.try_push_str("?")?;
        }
        Ok(ext)
    }

    fn get_fn_param(f: &Function) -> Result<String> {
        let mut param = String::try_from("(")?;
        for a in f.arg_names.into_iter().flatten() {
            if param.len() != 1 {
                param.try_push_str(", ")?;
            }
            if param.len() == 1 && a == "self" {
                continue;
            }
            param.try_push_str(a)?;
        }
        param.try_push(')')?;
        Ok(param)
    }

    fn get_fn_ret_typ(&self, f: &Function) -> Result<String> {
        let mut param = String::new();
        if !self.extensions {
            return Ok(param);
        }

        if let Some(item) = f
            .return_type
            .and_then(|h| self.ctx.meta_by_hash(h).ok())
            .and_then(|v| v.into_iter().next())
            .and_then(|m| m.item)
            .and_then(|i| i.last())
        {
            param.try_push_str(" -> ")?;
            param.try_push_str(&item.try_to_string()?)?;
        }

        Ok(param)
    }

    fn write_hint(
        &self,
        f: &mut dyn core::fmt::Write,
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
            write!(f, r#", docHTML: `{}`"#, doc)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }

    fn write_instances(&self, f: &mut dyn core::fmt::Write) -> Result<()> {
        write!(f, r#"var instance = ["#)?;

        let mut no_comma = true;
        for (item, meta) in self.instance.iter() {
            let Kind::Function(fnc) = meta.kind else {
                continue;
            };
            if no_comma {
                no_comma = false;
            } else {
                write!(f, r#","#)?;
            }

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
        }
        write!(f, "];")?;

        Ok(())
    }

    fn write_fixed(&self, f: &mut dyn core::fmt::Write) -> Result<()> {
        write!(f, r#"var fixed = ["#)?;

        let mut no_comma = true;
        for (item, meta) in self.fixed.iter() {
            if no_comma {
                no_comma = false;
            } else {
                write!(f, r#","#)?;
            }

            match meta.kind {
                Kind::Unsupported => {
                    no_comma = true;
                }
                Kind::Type => {
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Type", 0, None, doc.as_deref())?;
                }
                Kind::Struct => {
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Struct", 0, None, doc.as_deref())?;
                }
                Kind::Variant => {
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Variant", 0, None, doc.as_deref())?;
                }
                Kind::Enum => {
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Enum", 0, None, doc.as_deref())?;
                }
                Kind::Macro => {
                    let mut name = Self::get_name(item)?;
                    name.try_push_str("!()")?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Type", 0, None, doc.as_deref())?;
                }
                Kind::Function(fnc) => {
                    let mut value = Self::get_name(item)?;
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
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Const", 10, None, doc.as_deref())?;
                }
                Kind::Module => {
                    let name = Self::get_name(item)?;
                    let doc = self.doc_to_html(meta).ok().flatten();
                    self.write_hint(f, &name, "Module", 9, None, doc.as_deref())?;
                }
            }
        }
        write!(f, "];")?;

        Ok(())
    }

    fn write(&self, f: &mut dyn core::fmt::Write) -> Result<()> {
        write!(f, "{COMPLETER}\n\n")?;
        self.write_fixed(f)?;
        write!(f, "\n\n")?;
        self.write_instances(f)?;
        Ok(())
    }
}

impl<'a> Display for AutoCompleteCtx<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.write(f).map_err(|_| core::fmt::Error)
    }
}

static COMPLETER: &str = r#"
const runeCompleter = {
  getCompletions: (editor, session, pos, prefix, callback) => {
    if (prefix.length === 0) {
      callback(null, []);
      return;
    }

    var token = session.getTokenAt(pos.row, pos.column - 1).value;

    if (token.includes(".")) {
      callback(null, instance);
    } else {
      callback(null, fixed);
    }
  },
};
export default runeCompleter;
"#;

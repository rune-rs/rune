//! The `Printer` trait and implementations.

use core::mem::take;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::ast::{self, Span, Spanned};

use super::error::FormattingError;
use super::indent_writer::IndentedWriter;
use super::indent_writer::SpanInjectionWriter;

type Result<T> = core::result::Result<T, FormattingError>;

pub(super) struct Printer<'a> {
    writer: SpanInjectionWriter<'a>,
    source: &'a str,
}

impl<'a> Printer<'a> {
    pub(super) fn new(source: &'a str) -> Result<Self> {
        let writer = SpanInjectionWriter::new(IndentedWriter::new()?, source)?;
        Ok(Self { writer, source })
    }

    pub(super) fn commit(self) -> Result<Vec<u8>> {
        let inner = self.writer.into_inner()?;

        let mut out = Vec::new();

        let mut head = true;
        let mut lines = 0;

        for line in inner {
            if line.iter().all(|b| b.is_ascii_whitespace()) {
                lines += 1;
                continue;
            }

            if !take(&mut head) {
                out.try_resize(out.len().saturating_add(lines), b'\n')?;
            }

            out.try_extend(line)?;
            lines = 1;
        }

        if lines > 0 {
            out.try_push(b'\n')?;
        }

        Ok(out)
    }

    pub(super) fn resolve(&self, span: Span) -> Result<&'a str> {
        let Some(s) = self.source.get(span.range()) else {
            return Err(FormattingError::BadRange(span.range(), self.source.len()));
        };

        Ok(s)
    }

    pub(super) fn visit_file(&mut self, file: &ast::File) -> Result<()> {
        if let Some(shebang) = &file.shebang {
            self.writer.write_spanned_raw(shebang.span, true, false)?;
        }

        for attribute in &file.attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        for item in &file.items {
            self.visit_item(&item.0, item.1)?;
        }

        Ok(())
    }

    pub(super) fn visit_attribute(&mut self, attribute: &ast::Attribute) -> Result<bool> {
        let ast::Attribute {
            hash,
            style,
            open,
            path,
            input,
            close,
        } = attribute;

        let first = &path.first;
        if let ast::PathSegment::Ident(ident) = first {
            if let ast::LitSource::BuiltIn(ast::BuiltIn::Doc) = ident.source {
                self.writer.write_spanned_raw(ident.span, false, false)?;
                return Ok(true);
            }
        }

        self.writer.write_spanned_raw(hash.span, false, false)?;

        match style {
            ast::AttrStyle::Outer(bang) => {
                self.writer.write_spanned_raw(bang.span, false, false)?
            }
            ast::AttrStyle::Inner => {}
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_path(path)?;
        for token in input {
            self.writer.write_spanned_raw(token.span, false, false)?;
        }
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(false)
    }

    pub(super) fn visit_item(
        &mut self,
        item: &ast::Item,
        semi: Option<ast::SemiColon>,
    ) -> Result<()> {
        match item {
            ast::Item::Use(usage) => self.visit_use(usage, semi)?,
            ast::Item::Fn(item) => self.visit_fn(item, semi)?,
            ast::Item::Enum(item) => self.visit_enum(item, semi)?,
            ast::Item::Struct(item) => self.visit_struct(item, semi)?,
            ast::Item::Impl(item) => self.visit_impl(item, semi)?,
            ast::Item::Mod(item) => self.visit_mod(item, semi)?,
            ast::Item::Const(item) => self.visit_const(item, semi)?,
            ast::Item::MacroCall(item) => self.visit_macro_call(item, semi)?,
        }

        if !matches!(item, ast::Item::MacroCall(_)) {
            self.writer.newline()?;
        }

        Ok(())
    }

    fn visit_const(&mut self, ast: &ast::ItemConst, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemConst {
            id: _,
            attributes,
            visibility,
            const_token,
            name,
            eq,
            expr,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }
        self.writer.newline()?;

        self.emit_visibility(visibility)?;

        self.writer
            .write_spanned_raw(const_token.span, false, true)?;
        self.writer.write_spanned_raw(name.span, false, true)?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_mod(&mut self, item: &ast::ItemMod, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemMod {
            id: _,
            attributes,
            visibility,
            mod_token,
            name,
            body,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;

        self.writer.write_spanned_raw(mod_token.span, false, true)?;
        self.writer.write_spanned_raw(name.span, false, false)?;

        match body {
            ast::ItemModBody::EmptyBody(semi) => {
                self.writer.write_spanned_raw(semi.span, false, false)?;
            }
            ast::ItemModBody::InlineBody(body) => {
                self.writer.write_unspanned(" ")?;
                self.writer.write_spanned_raw(body.open.span, true, false)?;
                self.writer.indent();

                self.visit_file(&body.file)?;

                self.writer.dedent();
                self.writer
                    .write_spanned_raw(body.close.span, false, false)?;
            }
        }

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_impl(&mut self, item: &ast::ItemImpl, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemImpl {
            attributes,
            impl_,
            path,
            open,
            functions,
            close,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.writer.write_spanned_raw(impl_.span, false, true)?;
        self.visit_path(path)?;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(open.span, true, false)?;

        self.writer.indent();

        for function in functions {
            self.visit_fn(function, None)?;
            self.writer.newline()?;
        }

        self.writer.dedent();
        self.writer.write_spanned_raw(close.span, false, false)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_struct(&mut self, item: &ast::ItemStruct, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemStruct {
            id: _,
            attributes,
            visibility,
            struct_token,
            ident,
            body,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        self.writer
            .write_spanned_raw(struct_token.span, false, true)?;

        self.writer.write_spanned_raw(ident.span, false, false)?;

        self.visit_struct_body(body)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_struct_body(&mut self, ast: &ast::Fields) -> Result<()> {
        match ast {
            ast::Fields::Empty => {}
            ast::Fields::Unnamed(tuple) => {
                self.writer
                    .write_spanned_raw(tuple.open.span, false, false)?;
                for (field, comma) in tuple {
                    self.visit_field(field)?;
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, false)?;
                    }
                }
                self.writer
                    .write_spanned_raw(tuple.close.span, false, false)?;
            }
            ast::Fields::Named(body) => {
                self.writer.write_unspanned(" ")?;
                self.writer.write_spanned_raw(body.open.span, true, false)?;

                self.writer.indent();
                for (field, comma) in body {
                    self.visit_field(field)?;
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, true, false)?;
                    } else {
                        self.writer.write_unspanned(",\n")?;
                    }
                }
                self.writer.dedent();
                self.writer
                    .write_spanned_raw(body.close.span, false, false)?;
            }
        }

        Ok(())
    }

    fn visit_enum(&mut self, item: &ast::ItemEnum, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemEnum {
            attributes,
            visibility,
            enum_token,
            name,
            variants,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        self.writer
            .write_spanned_raw(enum_token.span, false, true)?;
        self.writer.write_spanned_raw(name.span, false, true)?;
        self.writer
            .write_spanned_raw(variants.open.span, true, false)?;

        self.writer.indent();
        for (variant, _comma) in variants {
            self.visit_variant(variant)?;
            let span = if let Some(comma) = _comma {
                comma.span
            } else {
                Span::new(0, 0)
            };
            self.writer.write_spanned_raw(span, true, false)?;
        }
        self.writer.dedent();
        self.writer
            .write_spanned_raw(variants.close.span, false, false)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_variant(&mut self, ast: &ast::ItemVariant) -> Result<()> {
        let ast::ItemVariant {
            id: _,
            attributes,
            name,
            body,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.writer.write_spanned_raw(name.span, false, false)?;

        self.visit_variant_body(body)?;

        Ok(())
    }

    fn visit_variant_body(&mut self, ast: &ast::Fields) -> Result<()> {
        match ast {
            ast::Fields::Empty => {}
            ast::Fields::Unnamed(body) => {
                self.writer
                    .write_spanned_raw(body.open.span, false, false)?;

                let count = body.parenthesized.len();
                for (idx, (field, comma)) in body.parenthesized.iter().enumerate() {
                    self.visit_field(field)?;
                    if idx < count - 1 {
                        if let Some(comma) = comma {
                            self.writer.write_spanned_raw(comma.span, false, true)?;
                        } else {
                            self.writer.write_unspanned(", ")?;
                        }
                    }
                }

                self.writer
                    .write_spanned_raw(body.close.span, false, false)?;
            }
            ast::Fields::Named(sbody) => {
                self.writer.write_unspanned(" ")?;
                self.writer
                    .write_spanned_raw(sbody.open.span, true, false)?;

                self.writer.indent();
                for (field, comma) in &sbody.braced {
                    self.visit_field(field)?;
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, true, false)?;
                    } else {
                        self.writer.write_unspanned(",\n")?;
                    }
                }
                self.writer.dedent();
                self.writer
                    .write_spanned_raw(sbody.close.span, false, false)?;
            }
        }

        Ok(())
    }

    fn visit_field(&mut self, ast: &ast::Field) -> Result<()> {
        let ast::Field {
            attributes,
            visibility,
            name,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        self.writer.write_spanned_raw(name.span, false, false)?;

        Ok(())
    }

    fn visit_fn(&mut self, item: &ast::ItemFn, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemFn {
            id: _,
            attributes,
            visibility,
            const_token,
            async_token,
            fn_token,
            name,
            args,
            body,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        if let Some(const_token) = const_token {
            self.writer
                .write_spanned_raw(const_token.span, false, true)?;
        }

        if let Some(async_token) = async_token {
            self.writer
                .write_spanned_raw(async_token.span, false, true)?;
        }

        self.writer.write_spanned_raw(fn_token.span, false, true)?;
        self.writer.write_spanned_raw(name.span, false, false)?;

        self.writer
            .write_spanned_raw(args.open.span, false, false)?;

        let multiline = if args.len() > 5 {
            self.writer.indent();
            self.writer.newline()?;
            true
        } else {
            false
        };

        for (arg, comma) in args {
            match arg {
                ast::FnArg::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
                ast::FnArg::Pat(pattern) => self.visit_pattern(pattern)?,
            }

            if let Some(comma) = comma {
                self.writer
                    .write_spanned_raw(comma.span, multiline, !multiline)?;
            }
        }

        if args.len() > 5 {
            self.writer.dedent();
            self.writer.newline()?;
        }

        self.writer
            .write_spanned_raw(args.close.span, false, true)?;
        self.visit_block(body)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_use(&mut self, usage: &ast::ItemUse, semi: Option<ast::SemiColon>) -> Result<()> {
        let ast::ItemUse {
            attributes,
            visibility,
            use_token,
            path,
        } = usage;
        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        self.writer.write_spanned_raw(use_token.span, false, true)?;
        self.visit_item_use_path(path, None)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_item_use_path(
        &mut self,
        path: &ast::ItemUsePath,
        comma: Option<ast::Comma>,
    ) -> Result<()> {
        let ast::ItemUsePath {
            global,
            first,
            segments,
            alias,
        } = path;

        if let Some(global) = global {
            self.writer.write_spanned_raw(global.span, false, false)?;
        }

        self.visit_item_use_segment(first)?;
        for (cc, segment) in segments {
            self.writer.write_spanned_raw(cc.span, false, false)?;

            self.visit_item_use_segment(segment)?;
        }

        if let Some((as_, ident)) = alias {
            self.writer.write_spanned_raw(as_.span, false, true)?;
            self.writer.write_spanned_raw(ident.span, false, false)?;
        }

        if let Some(comma) = comma {
            self.writer.write_spanned_raw(comma.span, false, true)?;
        }

        Ok(())
    }

    fn visit_path_segment(&mut self, segment: &ast::PathSegment) -> Result<()> {
        match segment {
            ast::PathSegment::SelfType(selftype) => self.visit_self_type(selftype)?,
            ast::PathSegment::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
            ast::PathSegment::Ident(ident) => {
                self.writer.write_spanned_raw(ident.span, false, false)?
            }
            ast::PathSegment::Crate(c) => self.writer.write_spanned_raw(c.span, false, false)?,
            ast::PathSegment::Super(s) => self.writer.write_spanned_raw(s.span, false, false)?,
            ast::PathSegment::Generics(g) => self.visit_generics(g)?,
        }
        Ok(())
    }

    fn visit_self_type(&mut self, selftype: &ast::SelfType) -> Result<()> {
        self.writer.write_spanned_raw(selftype.span, false, false)?;
        Ok(())
    }

    fn visit_self_value(&mut self, selfvalue: &ast::SelfValue) -> Result<()> {
        self.writer
            .write_spanned_raw(selfvalue.span, false, false)?;
        Ok(())
    }

    fn visit_generics(
        &mut self,
        generics: &ast::AngleBracketed<ast::PathSegmentExpr, ast::Comma>,
    ) -> Result<()> {
        self.writer
            .write_spanned_raw(generics.open.span, false, false)?;

        for (expr, comma) in generics {
            self.visit_path_segment_expr(expr)?;

            if let Some(comma) = comma {
                self.writer.write_spanned_raw(comma.span, false, true)?;
            }
        }

        self.writer
            .write_spanned_raw(generics.close.span, false, false)?;

        Ok(())
    }

    fn visit_expr(&mut self, expr: &ast::Expr) -> Result<()> {
        match expr {
            ast::Expr::Path(path) => self.visit_path(path),
            ast::Expr::Lit(lit) => self.visit_lit(lit),
            ast::Expr::Binary(binary) => self.visit_binary(binary),
            ast::Expr::Unary(unary) => self.visit_unary(unary),
            ast::Expr::Group(group) => self.visit_group(group),
            ast::Expr::Block(block) => self.visit_expr_block(block),
            ast::Expr::If(ifexpr) => self.visit_if(ifexpr),
            ast::Expr::While(whileexpr) => self.visit_while(whileexpr),
            ast::Expr::For(forexpr) => self.visit_for(forexpr),
            ast::Expr::Loop(loopexpr) => self.visit_loop(loopexpr),
            ast::Expr::Match(matchexpr) => self.visit_match(matchexpr),
            ast::Expr::Closure(closure) => self.visit_closure(closure),
            ast::Expr::Return(returnexpr) => self.visit_return(returnexpr),
            ast::Expr::Break(breakexpr) => self.visit_break(breakexpr),
            ast::Expr::Continue(continueexpr) => self.visit_continue(continueexpr),
            ast::Expr::Index(index) => self.visit_index(index),
            ast::Expr::Call(call) => self.visit_call(call),
            ast::Expr::FieldAccess(fieldaccess) => self.visit_field_access(fieldaccess),
            ast::Expr::Tuple(tuple) => self.visit_tuple(tuple),
            ast::Expr::Range(range) => self.visit_range(range),
            ast::Expr::Yield(yieldexpr) => self.visit_yield(yieldexpr),
            ast::Expr::Try(tri) => self.visit_try(tri),
            ast::Expr::Await(awaitexpr) => self.visit_await(awaitexpr),
            ast::Expr::Assign(assign) => self.visit_assign(assign),
            ast::Expr::Let(let_) => self.visit_let(let_),
            ast::Expr::Select(sel) => self.visit_select(sel),
            ast::Expr::Object(object) => self.visit_object(object),
            ast::Expr::Vec(vec) => self.visit_vec(vec),
            ast::Expr::Empty(empty) => self.visit_empty(empty),
            ast::Expr::MacroCall(macrocall) => self.visit_macro_call(macrocall, None),
        }
    }

    fn visit_macro_call(
        &mut self,
        macrocall: &ast::MacroCall,
        semi: Option<ast::SemiColon>,
    ) -> Result<()> {
        // Note: We don't visit the stream, as emitting it truthfully is quite hard and we can't format it. Instead we just resolve everything between the open/close.
        let ast::MacroCall {
            id: _,
            attributes,
            path,
            bang,
            open,
            input: _,
            close,
        } = macrocall;

        let first = &path.first;

        if let ast::PathSegment::Ident(ident) = first {
            if let ast::LitSource::BuiltIn(ast::BuiltIn::Template) = ident.source {
                let important_token = self.resolve(Span::new(open.span.end, close.span.start))?;
                write!(self.writer, "{}", important_token)?;
                return Ok(());
            }
        }

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_path(path)?;

        self.writer.write_spanned_raw(bang.span, false, false)?;
        self.writer.write_spanned_raw(open.span, false, false)?;
        self.writer
            .write_spanned_raw(Span::new(open.span.end, close.span.start), false, false)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        if let Some(semi) = semi {
            self.writer.write_spanned_raw(semi.span, true, false)?;
        }

        Ok(())
    }

    fn visit_empty(&mut self, ast: &ast::ExprEmpty) -> Result<()> {
        let ast::ExprEmpty {
            attributes,
            open,
            expr,
            close,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_vec(&mut self, ast: &ast::ExprVec) -> Result<()> {
        let ast::ExprVec { attributes, items } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(items.open.span, false, false)?;

        let multiline = if items.len() > 10 {
            self.writer.indent();
            self.writer.newline()?;
            true
        } else {
            false
        };

        let count = items.len();
        for (idx, (item, comma)) in items.iter().enumerate() {
            self.visit_expr(item)?;

            if multiline {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, true, false)?;
                }
            } else {
                let is_last = count == idx + 1;
                if !is_last {
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, true)?;
                    } else {
                        write!(self.writer, ", ")?;
                    }
                }
            }
        }

        if multiline {
            self.writer.dedent();
        }

        self.writer
            .write_spanned_raw(items.close.span, false, false)?;
        Ok(())
    }

    fn visit_object(&mut self, ast: &ast::ExprObject) -> Result<()> {
        let ast::ExprObject {
            attributes,
            ident,
            assignments,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        match ident {
            ast::ObjectIdent::Anonymous(p) => {
                self.writer.write_spanned_raw(p.span, false, false)?;
            }
            ast::ObjectIdent::Named(named) => {
                self.visit_path(named)?;
                self.writer.write_unspanned(" ")?;
            }
        }

        self.writer
            .write_spanned_raw(assignments.open.span, false, false)?;

        let has_items = !assignments.is_empty();
        let multiline = if assignments.len() > 5 {
            self.writer.indent();
            self.writer.newline()?;
            true
        } else {
            if has_items {
                write!(self.writer, " ")?;
            }
            false
        };

        let count = assignments.len();
        for (idx, (assignment, comma)) in assignments.iter().enumerate() {
            self.visit_object_assignment(assignment)?;

            if multiline {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, true, false)?;
                } else {
                    self.writer.write_unspanned(",\n")?;
                }
            } else {
                let is_last = count == idx + 1;
                if !is_last {
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, true)?;
                    } else {
                        write!(self.writer, ", ")?;
                    }
                }
            }
        }

        if multiline {
            self.writer.dedent();
            self.writer.newline()?;
        } else if has_items {
            self.writer.write_unspanned(" ")?;
        }

        self.writer
            .write_spanned_raw(assignments.close.span, false, false)?;

        Ok(())
    }

    fn visit_object_assignment(&mut self, ast: &ast::FieldAssign) -> Result<()> {
        let ast::FieldAssign { key, assign } = ast;

        match key {
            ast::ObjectKey::LitStr(key) => {
                self.writer.write_spanned_raw(key.span, false, false)?;
            }
            ast::ObjectKey::Path(path) => self.visit_path(path)?,
        }

        if let Some((colon, assign)) = assign {
            self.writer.write_spanned_raw(colon.span, false, true)?;
            self.visit_expr(assign)?;
        }

        Ok(())
    }

    fn visit_select(&mut self, ast: &ast::ExprSelect) -> Result<()> {
        let ast::ExprSelect {
            attributes,
            select,
            open,
            branches,
            close,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer.write_spanned_raw(select.span, false, true)?;
        self.writer.write_spanned_raw(open.span, true, false)?;
        self.writer.indent();
        for (branch, comma) in branches {
            self.visit_select_branch(branch)?;
            if let Some(comma) = comma {
                self.writer.write_spanned_raw(comma.span, true, false)?;
            } else {
                self.writer.write_unspanned(",\n")?;
            }
        }
        self.writer.dedent();

        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_select_branch(&mut self, ast: &ast::ExprSelectBranch) -> Result<()> {
        match ast {
            ast::ExprSelectBranch::Pat(pat) => self.visit_select_pattern(pat)?,
            ast::ExprSelectBranch::Default(_default) => write!(self.writer, "default")?,
        }

        Ok(())
    }

    fn visit_select_pattern(&mut self, ast: &ast::ExprSelectPatBranch) -> Result<()> {
        let ast::ExprSelectPatBranch {
            pat,
            eq,
            expr,
            rocket,
            body,
        } = ast;

        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(rocket.span, false, true)?;
        self.visit_expr(body)?;

        Ok(())
    }

    fn visit_assign(&mut self, ast: &ast::ExprAssign) -> Result<()> {
        let ast::ExprAssign {
            attributes,
            lhs,
            eq,
            rhs,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(lhs)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(rhs)?;

        Ok(())
    }

    fn visit_await(&mut self, ast: &ast::ExprAwait) -> Result<()> {
        let ast::ExprAwait {
            attributes,
            expr,
            dot,
            await_token,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(dot.span, false, false)?;
        self.writer
            .write_spanned_raw(await_token.span, false, false)?;

        Ok(())
    }

    fn visit_try(&mut self, ast: &ast::ExprTry) -> Result<()> {
        let ast::ExprTry {
            attributes,
            expr,
            try_token,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer
            .write_spanned_raw(try_token.span, false, false)?;

        Ok(())
    }

    fn visit_yield(&mut self, ast: &ast::ExprYield) -> Result<()> {
        let ast::ExprYield {
            attributes,
            expr,
            yield_token,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(yield_token.span, false, false)?;

        if let Some(expr) = expr {
            self.writer.write_unspanned(" ")?;
            self.visit_expr(expr)?;
        }

        Ok(())
    }

    fn visit_range(&mut self, ast: &ast::ExprRange) -> Result<()> {
        let ast::ExprRange {
            attributes,
            start: from,
            limits,
            end: to,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some(from) = from {
            self.visit_expr(from)?;
        }

        match limits {
            ast::ExprRangeLimits::HalfOpen(_) => write!(self.writer, "..")?,
            ast::ExprRangeLimits::Closed(_) => write!(self.writer, "..=")?,
        }

        if let Some(to) = to {
            self.visit_expr(to)?;
        }

        Ok(())
    }

    fn visit_tuple(&mut self, ast: &ast::ExprTuple) -> Result<()> {
        let ast::ExprTuple { attributes, items } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
            self.writer.newline()?;
        }

        self.writer
            .write_spanned_raw(items.open.span, false, false)?;

        let multiline = if items.len() >= 5 {
            self.writer.indent();
            self.writer.newline()?;
            true
        } else {
            false
        };

        let count = items.len();
        for (idx, (item, comma)) in items.iter().enumerate() {
            self.visit_expr(item)?;
            if multiline {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, true, false)?;
                } else {
                    self.writer.write_unspanned(",\n")?;
                }
            } else {
                let is_last = idx == count - 1;
                if !is_last {
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, true)?;
                    } else {
                        write!(self.writer, ", ")?;
                    }
                }
            }
        }

        if multiline {
            self.writer.dedent();
        }

        self.writer
            .write_spanned_raw(items.close.span, false, false)?;

        Ok(())
    }

    fn visit_field_access(&mut self, ast: &ast::ExprFieldAccess) -> Result<()> {
        let ast::ExprFieldAccess {
            attributes,
            expr,
            dot,
            expr_field,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(dot.span, false, false)?;
        self.visit_expr_field(expr_field)?;

        Ok(())
    }

    fn visit_expr_field(&mut self, ast: &ast::ExprField) -> Result<()> {
        match ast {
            ast::ExprField::Path(path) => self.visit_path(path)?,
            ast::ExprField::LitNumber(num) => {
                self.writer.write_spanned_raw(num.span, false, false)?
            }
        }

        Ok(())
    }

    fn visit_call(&mut self, ast: &ast::ExprCall) -> Result<()> {
        let ast::ExprCall {
            id: _,
            attributes,
            expr,
            args,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer
            .write_spanned_raw(args.open.span, false, false)?;

        let count = args.parenthesized.len();
        for (idx, (arg, comma)) in args.parenthesized.iter().enumerate() {
            self.visit_expr(arg)?;
            if idx != count - 1 {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, false, true)?;
                } else {
                    write!(self.writer, ", ")?;
                }
            }
        }

        self.writer
            .write_spanned_raw(args.close.span, false, false)?;

        Ok(())
    }

    fn visit_index(&mut self, ast: &ast::ExprIndex) -> Result<()> {
        let ast::ExprIndex {
            attributes,
            target,
            open,
            index,
            close,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(target)?;
        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(index)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_continue(&mut self, ast: &ast::ExprContinue) -> Result<()> {
        let ast::ExprContinue {
            attributes,
            continue_token,
            label,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(continue_token.span, false, false)?;

        if let Some(label) = label {
            self.writer.write_unspanned(" ")?;
            self.writer.write_spanned_raw(label.span, false, false)?;
        }

        Ok(())
    }

    fn visit_break(&mut self, ast: &ast::ExprBreak) -> Result<()> {
        let ast::ExprBreak {
            attributes,
            break_token,
            label,
            expr,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(break_token.span, false, false)?;

        if let Some(label) = label {
            self.writer.write_unspanned(" ")?;
            self.writer.write_spanned_raw(label.span, false, false)?;
        }

        if let Some(expr) = expr {
            self.writer.write_unspanned(" ")?;
            self.visit_expr(expr)?;
        }

        Ok(())
    }

    fn visit_return(&mut self, ast: &ast::ExprReturn) -> Result<()> {
        let ast::ExprReturn {
            attributes,
            return_token,
            expr,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(return_token.span, false, false)?;

        if let Some(expr) = expr {
            self.writer.write_unspanned(" ")?;
            self.visit_expr(expr)?;
        }

        Ok(())
    }

    fn visit_closure(&mut self, ast: &ast::ExprClosure) -> Result<()> {
        let ast::ExprClosure {
            id: _,
            attributes,
            async_token,
            move_token,
            args,
            body,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some(async_token) = async_token {
            self.writer
                .write_spanned_raw(async_token.span, false, true)?;
        }

        if let Some(move_token) = move_token {
            self.writer
                .write_spanned_raw(move_token.span, false, true)?;
        }

        match args {
            ast::ExprClosureArgs::Empty { token } => {
                self.writer.write_spanned_raw(token.span, false, true)?
            }
            ast::ExprClosureArgs::List { args, open, close } => {
                self.writer.write_spanned_raw(open.span, false, false)?;
                for (arg, comma) in args {
                    match arg {
                        ast::FnArg::SelfValue(self_) => self.visit_self_value(self_)?,
                        ast::FnArg::Pat(pat) => self.visit_pattern(pat)?,
                    }
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, true)?;
                    }
                }

                self.writer.write_spanned_raw(close.span, false, true)?;
            }
        }

        self.visit_expr(body)?;

        Ok(())
    }

    fn visit_match(&mut self, ast: &ast::ExprMatch) -> Result<()> {
        let ast::ExprMatch {
            attributes,
            match_,
            expr,
            open,
            branches,
            close,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer.write_spanned_raw(match_.span, false, true)?;
        self.visit_expr(expr)?;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(open.span, true, false)?;

        self.writer.indent();
        for (branch, comma) in branches {
            let should_have_comma = self.visit_match_branch(branch)?;

            if should_have_comma {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, true, false)?;
                } else {
                    self.writer.write_unspanned(",\n")?;
                }
            } else {
                self.writer.write_unspanned("\n")?;
            }
        }
        self.writer.dedent();

        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_match_branch(&mut self, ast: &ast::ExprMatchBranch) -> Result<bool> {
        let ast::ExprMatchBranch {
            pat,
            condition,
            rocket,
            body,
        } = ast;

        self.visit_pattern(pat)?;

        if let Some((if_, expr)) = condition {
            self.writer.write_unspanned(" ")?;
            self.writer.write_spanned_raw(if_.span, false, true)?;
            self.visit_expr(expr)?;
        }
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(rocket.span, false, true)?;
        self.visit_expr(body)?;

        let should_have_comma = !matches!(body, ast::Expr::Block(_));

        Ok(should_have_comma)
    }

    fn visit_loop(&mut self, ast: &ast::ExprLoop) -> Result<()> {
        let ast::ExprLoop {
            attributes,
            label,
            loop_token,
            body,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some((label, colon)) = label {
            self.writer.write_spanned_raw(label.span, false, false)?;
            self.writer.write_spanned_raw(colon.span, false, true)?;
        }

        self.writer
            .write_spanned_raw(loop_token.span, false, true)?;

        self.visit_block(body)?;

        Ok(())
    }

    fn visit_for(&mut self, ast: &ast::ExprFor) -> Result<()> {
        let ast::ExprFor {
            attributes,
            label,
            binding,
            in_,
            iter,
            body,
            for_token,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some((label, colon)) = label {
            self.writer.write_spanned_raw(label.span, false, false)?;
            self.writer.write_spanned_raw(colon.span, false, true)?;
        }

        self.writer.write_spanned_raw(for_token.span, false, true)?;

        self.visit_pattern(binding)?;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(in_.span, false, true)?;

        self.visit_expr(iter)?;
        self.writer.write_unspanned(" ")?;

        self.visit_block(body)?;

        Ok(())
    }

    fn visit_while(&mut self, ast: &ast::ExprWhile) -> Result<()> {
        let ast::ExprWhile {
            attributes,
            label,
            while_token,
            condition,
            body,
        } = ast;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some((label, colon)) = label {
            self.writer.write_spanned_raw(label.span, false, false)?;
            self.writer.write_spanned_raw(colon.span, false, true)?;
        }

        self.writer
            .write_spanned_raw(while_token.span, false, true)?;
        self.visit_condition(condition)?;
        self.writer.write_unspanned(" ")?;
        self.visit_block(body)?;

        Ok(())
    }

    fn visit_condition(&mut self, ast: &ast::Condition) -> Result<()> {
        match ast {
            ast::Condition::Expr(expr) => self.visit_expr(expr),
            ast::Condition::ExprLet(let_) => self.visit_let(let_),
        }
    }

    fn visit_pattern(&mut self, ast: &ast::Pat) -> Result<()> {
        match ast {
            ast::Pat::Ignore(ignore) => self.visit_pat_ignore(ignore)?,
            ast::Pat::Path(path) => self.visit_pat_path(path)?,
            ast::Pat::Lit(patit) => self.visit_pat_lit(patit)?,
            ast::Pat::Vec(patvec) => self.visit_pat_vec(patvec)?,
            ast::Pat::Tuple(pattuple) => self.visit_pat_tuple(pattuple)?,
            ast::Pat::Object(ast) => self.visit_pat_object(ast)?,
            ast::Pat::Binding(binding) => self.visit_pat_binding(binding)?,
            ast::Pat::Rest(rest) => self.visit_pat_rest(rest)?,
        }

        Ok(())
    }

    fn visit_pat_rest(&mut self, ast: &ast::PatRest) -> Result<()> {
        let ast::PatRest {
            attributes,
            dot_dot,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(dot_dot.span, false, false)?;

        Ok(())
    }

    fn visit_pat_binding(&mut self, ast: &ast::PatBinding) -> Result<()> {
        let ast::PatBinding {
            attributes,
            key,
            colon,
            pat,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        match key {
            ast::ObjectKey::LitStr(str_) => {
                self.writer.write_spanned_raw(str_.span, false, false)?;
            }
            ast::ObjectKey::Path(path) => self.visit_path(path)?,
        }

        self.writer.write_spanned_raw(colon.span, false, true)?;

        self.visit_pattern(pat)?;

        Ok(())
    }

    fn visit_pat_object(&mut self, ast: &ast::PatObject) -> Result<()> {
        let ast::PatObject {
            attributes,
            ident,
            items,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        match ident {
            ast::ObjectIdent::Anonymous(pound) => {
                self.writer.write_spanned_raw(pound.span, false, false)?;
            }
            ast::ObjectIdent::Named(n) => {
                self.visit_path(n)?;
                self.writer.write_unspanned(" ")?;
            }
        }

        let ast::Braced {
            open,
            braced,
            close,
        } = items;

        let multiline = items.len() > 5;
        self.writer
            .write_spanned_raw(open.span, false, !multiline)?;

        if multiline {
            self.writer.newline()?;
            self.writer.indent();
        }

        let count = items.len();
        for (idx, (pat, comma)) in braced.iter().enumerate() {
            self.visit_pattern(pat)?;

            if multiline {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, true, false)?;
                } else {
                    self.writer.write_unspanned(",\n")?;
                }
            } else if idx < count - 1 {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, false, true)?;
                } else {
                    self.writer.write_unspanned(",\n")?;
                }
            }
        }

        if multiline {
            self.writer.dedent();
        } else {
            self.writer.write_unspanned(" ")?;
        }

        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_pat_tuple(&mut self, ast: &ast::PatTuple) -> Result<()> {
        let ast::PatTuple {
            attributes,
            items,
            path,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        if let Some(path) = path {
            self.visit_path(path)?;
        }

        self.writer
            .write_spanned_raw(items.open.span, false, false)?;

        for (pat, comma) in items {
            self.visit_pattern(pat)?;
            if let Some(comma) = comma {
                self.writer.write_spanned_raw(comma.span, false, true)?;
            }
        }

        self.writer
            .write_spanned_raw(items.close.span, false, false)?;

        Ok(())
    }

    fn visit_pat_vec(&mut self, ast: &ast::PatVec) -> Result<()> {
        let ast::PatVec { attributes, items } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer
            .write_spanned_raw(items.open.span, false, false)?;

        let count = items.len();
        for (idx, (pat, comma)) in items.iter().enumerate() {
            self.visit_pattern(pat)?;
            if idx < count - 1 {
                if let Some(comma) = comma {
                    self.writer.write_spanned_raw(comma.span, false, true)?;
                } else {
                    self.writer.write_unspanned(", ")?;
                }
            }
        }

        self.writer
            .write_spanned_raw(items.close.span, false, false)?;

        Ok(())
    }

    fn visit_pat_lit(&mut self, ast: &ast::PatLit) -> Result<()> {
        let ast::PatLit { attributes, expr } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_expr(expr)?;

        Ok(())
    }

    fn visit_pat_ignore(&mut self, ast: &ast::PatIgnore) -> Result<()> {
        let ast::PatIgnore {
            attributes,
            underscore,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer
            .write_spanned_raw(underscore.span, false, false)?;

        Ok(())
    }

    fn visit_pat_path(&mut self, ast: &ast::PatPath) -> Result<()> {
        let ast::PatPath { attributes, path } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_path(path)?;
        Ok(())
    }

    fn visit_let(&mut self, ast: &ast::ExprLet) -> Result<()> {
        let ast::ExprLet {
            attributes,
            let_token,
            mut_token,
            pat,
            eq,
            expr,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(let_token.span, false, true)?;

        if let Some(mut_token) = mut_token {
            self.writer.write_spanned_raw(mut_token.span, false, true)?;
        }

        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        Ok(())
    }

    fn visit_if(&mut self, ast: &ast::ExprIf) -> Result<()> {
        let ast::ExprIf {
            attributes,
            if_,
            condition,
            block,
            expr_else_ifs,
            expr_else,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(if_.span, false, true)?;
        self.visit_condition(condition)?;
        self.writer.write_unspanned(" ")?;
        self.visit_block(block)?;

        for expr_else_if in expr_else_ifs {
            self.visit_expr_else_if(expr_else_if)?;
        }

        if let Some(expr_else) = expr_else {
            self.visit_expr_else(expr_else)?;
        }

        Ok(())
    }

    fn visit_expr_else_if(&mut self, ast: &ast::ExprElseIf) -> Result<()> {
        let ast::ExprElseIf {
            else_,
            if_,
            condition,
            block,
        } = ast;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(else_.span, false, true)?;
        self.writer.write_spanned_raw(if_.span, false, true)?;

        self.visit_condition(condition)?;
        write!(self.writer, " ")?;
        self.visit_block(block)?;

        Ok(())
    }

    fn visit_expr_else(&mut self, ast: &ast::ExprElse) -> Result<()> {
        let ast::ExprElse { else_, block } = ast;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(else_.span, false, true)?;

        self.visit_block(block)?;
        Ok(())
    }

    fn visit_expr_block(&mut self, ast: &ast::ExprBlock) -> Result<()> {
        let ast::ExprBlock {
            attributes,
            async_token,
            const_token,
            move_token,
            block,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        if let Some(async_token) = async_token {
            self.writer
                .write_spanned_raw(async_token.span, false, true)?;
        }

        if let Some(const_token) = const_token {
            self.writer
                .write_spanned_raw(const_token.span, false, true)?;
        }

        if let Some(move_token) = move_token {
            self.writer
                .write_spanned_raw(move_token.span, false, true)?;
        }

        self.visit_block(block)
    }

    fn visit_block(&mut self, ast: &ast::Block) -> Result<()> {
        let ast::Block {
            id: _,
            open,
            statements,
            close,
        } = ast;

        self.writer.write_spanned_raw(open.span, true, false)?;

        self.writer.indent();
        for statement in statements {
            self.visit_statement(statement)?;
        }

        self.writer.dedent();
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_statement(&mut self, ast: &ast::Stmt) -> Result<()> {
        match ast {
            ast::Stmt::Local(local) => {
                self.visit_local(local)?;
                self.writer.newline()?;
            }
            ast::Stmt::Item(item, semi) => {
                self.visit_item(item, *semi)?;
                if !matches!(item, ast::Item::Fn(_)) {
                    self.writer.newline()?;
                }
            }
            ast::Stmt::Expr(expr) => {
                self.visit_expr(expr)?;
                self.writer.newline()?;
            }
            ast::Stmt::Semi(semi) => {
                let ast::StmtSemi { expr, semi_token } = semi;

                self.visit_expr(expr)?;
                self.writer
                    .write_spanned_raw(semi_token.span, false, false)?;
                self.writer.newline()?;
            }
        }

        Ok(())
    }

    fn visit_local(&mut self, ast: &ast::Local) -> Result<()> {
        let ast::Local {
            attributes,
            let_token,
            mut_token,
            pat,
            eq,
            expr,
            semi,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(let_token.span, false, true)?;

        if let Some(mut_token) = mut_token {
            self.writer.write_spanned_raw(mut_token.span, false, true)?;
        }

        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(semi.span, false, false)?;

        Ok(())
    }

    fn visit_unary(&mut self, ast: &ast::ExprUnary) -> Result<()> {
        let ast::ExprUnary {
            op,
            expr,
            attributes,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(op.span(), false, false)?;

        self.visit_expr(expr)
    }

    fn visit_group(&mut self, ast: &ast::ExprGroup) -> Result<()> {
        let ast::ExprGroup {
            attributes,
            open,
            expr,
            close,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_path(&mut self, path: &ast::Path) -> Result<()> {
        let ast::Path {
            id: _,
            global,
            first,
            rest,
            trailing,
        } = path;

        if let Some(global) = global {
            self.writer.write_spanned_raw(global.span, false, false)?;
        }

        self.visit_path_segment(first)?;
        for (cc, segment) in rest {
            self.writer.write_spanned_raw(cc.span, false, false)?;
            self.visit_path_segment(segment)?;
        }

        if let Some(trailing) = trailing {
            self.writer.write_spanned_raw(trailing.span, false, false)?;
        }

        Ok(())
    }

    fn visit_lit(&mut self, lit: &ast::ExprLit) -> Result<()> {
        let ast::ExprLit { attributes, lit } = lit;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        match lit {
            ast::Lit::Bool(b) => {
                self.writer.write_spanned_raw(b.span, false, false)?;
            }
            ast::Lit::Byte(val) => {
                self.writer.write_spanned_raw(val.span, false, false)?;
            }
            ast::Lit::Str(v) => {
                self.writer.write_spanned_raw(v.span, false, false)?;
            }
            ast::Lit::ByteStr(v) => {
                self.writer.write_spanned_raw(v.span, false, false)?;
            }
            ast::Lit::Char(c) => {
                self.writer.write_spanned_raw(c.span, false, false)?;
            }
            ast::Lit::Number(n) => {
                self.writer.write_spanned_raw(n.span, false, false)?;
            }
        }
        Ok(())
    }

    fn visit_binary(&mut self, ast: &ast::ExprBinary) -> Result<()> {
        let ast::ExprBinary {
            attributes,
            op,
            lhs,
            rhs,
        } = ast;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_expr(lhs.as_ref())?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(op.span(), false, true)?;
        self.visit_expr(rhs.as_ref())?;
        Ok(())
    }

    fn visit_path_segment_expr(&mut self, expr: &ast::PathSegmentExpr) -> Result<()> {
        let ast::PathSegmentExpr { expr } = expr;
        self.visit_expr(expr)
    }

    fn visit_item_use_segment(&mut self, segment: &ast::ItemUseSegment) -> Result<()> {
        match segment {
            ast::ItemUseSegment::PathSegment(path) => {
                self.visit_path_segment(path)?;
            }
            ast::ItemUseSegment::Wildcard(star) => {
                self.writer.write_spanned_raw(star.span, false, false)?;
            }
            ast::ItemUseSegment::Group(braced_group) => {
                let ast::Braced {
                    open,
                    braced,
                    close,
                } = braced_group;

                self.writer.write_spanned_raw(open.span, false, false)?;

                for (item, comma) in braced {
                    self.visit_item_use_path(item, *comma)?;
                    if let Some(comma) = comma {
                        self.writer.write_spanned_raw(comma.span, false, true)?;
                    } else {
                        self.writer.write_unspanned(", ")?;
                    }
                }

                self.writer.write_spanned_raw(close.span, false, false)?;
            }
        }

        Ok(())
    }

    fn emit_visibility(&mut self, visibility: &ast::Visibility) -> Result<()> {
        match visibility {
            ast::Visibility::Public(p) => self.writer.write_spanned_raw(p.span, false, true)?,
            ast::Visibility::Inherited => {}
            ast::Visibility::Crate(c) => {
                self.writer
                    .write_spanned_raw(c.pub_token.span, false, false)?;
                self.writer.write_spanned_raw(c.open.span, false, false)?;
                self.writer
                    .write_spanned_raw(c.restriction.span, false, false)?;
                self.writer.write_spanned_raw(c.close.span, false, false)?;
            }
            ast::Visibility::Super(s) => {
                self.writer
                    .write_spanned_raw(s.pub_token.span, false, false)?;
                self.writer.write_spanned_raw(s.open.span, false, false)?;
                self.writer
                    .write_spanned_raw(s.restriction.span, false, false)?;
                self.writer.write_spanned_raw(s.close.span, false, false)?;
            }
            ast::Visibility::SelfValue(s) => {
                self.writer
                    .write_spanned_raw(s.pub_token.span, false, false)?;
                self.writer.write_spanned_raw(s.open.span, false, false)?;
                self.writer
                    .write_spanned_raw(s.restriction.span, false, false)?;
                self.writer.write_spanned_raw(s.close.span, false, false)?;
            }
            ast::Visibility::In(target) => {
                self.writer
                    .write_spanned_raw(target.pub_token.span, false, false)?;
                self.writer
                    .write_spanned_raw(target.open.span, false, false)?;
                self.writer
                    .write_spanned_raw(target.restriction.in_token.span, false, true)?;
                self.visit_path(&target.restriction.path)?;
                self.writer
                    .write_spanned_raw(target.close.span, false, false)?;
            }
        }

        Ok(())
    }
}

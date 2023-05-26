//! The `Printer` trait and implementations.

use core::mem::take;

use crate::no_std::io::Write;
use crate::no_std::prelude::*;

use crate::ast::{
    self, AngleBracketed, AttrStyle, Block, Braced, BuiltIn, Comma, Condition, Expr, ExprAssign,
    ExprAwait, ExprBinary, ExprBlock, ExprBreak, ExprBreakValue, ExprCall, ExprClosure,
    ExprClosureArgs, ExprContinue, ExprElse, ExprElseIf, ExprEmpty, ExprField, ExprFieldAccess,
    ExprFor, ExprGroup, ExprIf, ExprIndex, ExprLet, ExprLit, ExprLoop, ExprMatch, ExprMatchBranch,
    ExprObject, ExprRange, ExprReturn, ExprSelect, ExprSelectBranch, ExprSelectPatBranch, ExprTry,
    ExprTuple, ExprUnary, ExprVec, ExprWhile, ExprYield, Field, FieldAssign, Fields, FnArg, Item,
    ItemConst, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemModBody, ItemStruct, ItemVariant,
    LitSource, Local, MacroCall, ObjectKey, Pat, PatBinding, PatIgnore, PatLit, PatObject, PatPath,
    PatRest, PatTuple, PatVec, Path, PathSegment, PathSegmentExpr, SelfType, SelfValue, SemiColon,
    Span, Spanned, Stmt, StmtSemi,
};
use crate::Source;

use super::error::FormattingError;
use super::indent_writer::IndentedWriter;
use super::indent_writer::SpanInjectionWriter;

type Result<T> = core::result::Result<T, FormattingError>;

pub(super) struct Printer<'a> {
    writer: SpanInjectionWriter<'a>,
    source: &'a Source,
}

impl<'a> Printer<'a> {
    pub(super) fn new(source: &'a Source) -> Result<Self> {
        let writer = SpanInjectionWriter::new(IndentedWriter::new(), source)?;
        Ok(Self { writer, source })
    }

    pub(super) fn commit(self) -> Vec<u8> {
        let inner = self.writer.into_inner();

        let mut out = Vec::new();

        let mut head = true;
        let mut lines = 0;

        for line in inner {
            if line.iter().all(|b| b.is_ascii_whitespace()) {
                lines += 1;
                continue;
            }

            if !take(&mut head) {
                out.resize(out.len().saturating_add(lines), b'\n');
            }

            out.extend(line);
            lines = 1;
        }

        if lines > 0 {
            out.push(b'\n');
        }

        out
    }

    pub(super) fn resolve(&self, span: Span) -> Result<&'a str> {
        let Some(s) = self.source.get(span.range()) else {
            return Err(FormattingError::InvalidSpan(
                span.start.into_usize(),
                span.end.into_usize(),
                self.source.len(),
            ));
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
        if let PathSegment::Ident(ident) = first {
            if let LitSource::BuiltIn(BuiltIn::Doc) = ident.source {
                self.writer.write_spanned_raw(ident.span, false, false)?;
                return Ok(true);
            }
        }

        self.writer.write_spanned_raw(hash.span, false, false)?;

        match style {
            AttrStyle::Outer(bang) => self.writer.write_spanned_raw(bang.span, false, false)?,
            AttrStyle::Inner => {}
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_path(path)?;
        for token in input {
            self.writer.write_spanned_raw(token.span, false, false)?;
        }
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(false)
    }

    pub(super) fn visit_item(&mut self, item: &ast::Item, semi: Option<SemiColon>) -> Result<()> {
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

    fn visit_const(&mut self, item: &ItemConst, semi: Option<SemiColon>) -> Result<()> {
        let ItemConst {
            id: _,
            attributes,
            visibility,
            const_token,
            name,
            eq,
            expr,
        } = item;

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

    fn visit_mod(&mut self, item: &ItemMod, semi: Option<SemiColon>) -> Result<()> {
        let ItemMod {
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
            ItemModBody::EmptyBody(semi) => {
                self.writer.write_spanned_raw(semi.span, false, false)?;
            }
            ItemModBody::InlineBody(body) => {
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

    fn visit_impl(&mut self, item: &ItemImpl, semi: Option<SemiColon>) -> Result<()> {
        let ItemImpl {
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

    fn visit_struct(&mut self, item: &ItemStruct, semi: Option<SemiColon>) -> Result<()> {
        let ItemStruct {
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

    fn visit_struct_body(&mut self, body: &Fields) -> Result<()> {
        match body {
            Fields::Empty => {}
            Fields::Unnamed(tuple) => {
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
            Fields::Named(body) => {
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

    fn visit_enum(&mut self, item: &ItemEnum, semi: Option<SemiColon>) -> Result<()> {
        let ItemEnum {
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

    fn visit_variant(&mut self, variant: &ItemVariant) -> Result<()> {
        let ItemVariant {
            id: _,
            attributes,
            name,
            body,
        } = variant;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.writer.write_spanned_raw(name.span, false, false)?;

        self.visit_variant_body(body)?;

        Ok(())
    }

    fn visit_variant_body(&mut self, body: &Fields) -> Result<()> {
        match body {
            Fields::Empty => {}
            Fields::Unnamed(body) => {
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
            Fields::Named(sbody) => {
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

    fn visit_field(&mut self, field: &Field) -> Result<()> {
        let Field {
            attributes,
            visibility,
            name,
        } = field;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
            self.writer.newline()?;
        }

        self.emit_visibility(visibility)?;
        self.writer.write_spanned_raw(name.span, false, false)?;

        Ok(())
    }

    fn visit_fn(&mut self, item: &ItemFn, semi: Option<SemiColon>) -> Result<()> {
        let ItemFn {
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
                FnArg::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
                FnArg::Pat(pattern) => self.visit_pattern(pattern)?,
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

    fn visit_use(&mut self, usage: &ast::ItemUse, semi: Option<SemiColon>) -> Result<()> {
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

    fn visit_item_use_path(&mut self, path: &ast::ItemUsePath, comma: Option<Comma>) -> Result<()> {
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
            PathSegment::SelfType(selftype) => self.visit_self_type(selftype)?,
            PathSegment::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
            PathSegment::Ident(ident) => self.writer.write_spanned_raw(ident.span, false, false)?,
            PathSegment::Crate(c) => self.writer.write_spanned_raw(c.span, false, false)?,
            PathSegment::Super(s) => self.writer.write_spanned_raw(s.span, false, false)?,
            PathSegment::Generics(g) => self.visit_generics(g)?,
        }
        Ok(())
    }

    fn visit_self_type(&mut self, selftype: &SelfType) -> Result<()> {
        self.writer.write_spanned_raw(selftype.span, false, false)?;
        Ok(())
    }

    fn visit_self_value(&mut self, selfvalue: &SelfValue) -> Result<()> {
        self.writer
            .write_spanned_raw(selfvalue.span, false, false)?;
        Ok(())
    }

    fn visit_generics(&mut self, generics: &AngleBracketed<PathSegmentExpr, Comma>) -> Result<()> {
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

    fn visit_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Path(path) => self.visit_path(path),
            Expr::Lit(lit) => self.visit_lit(lit),
            Expr::Binary(binary) => self.visit_binary(binary),
            Expr::Unary(unary) => self.visit_unary(unary),
            Expr::Group(group) => self.visit_group(group),
            Expr::Block(block) => self.visit_expr_block(block),
            Expr::If(ifexpr) => self.visit_if(ifexpr),
            Expr::While(whileexpr) => self.visit_while(whileexpr),
            Expr::For(forexpr) => self.visit_for(forexpr),
            Expr::Loop(loopexpr) => self.visit_loop(loopexpr),
            Expr::Match(matchexpr) => self.visit_match(matchexpr),
            Expr::Closure(closure) => self.visit_closure(closure),
            Expr::Return(returnexpr) => self.visit_return(returnexpr),
            Expr::Break(breakexpr) => self.visit_break(breakexpr),
            Expr::Continue(continueexpr) => self.visit_continue(continueexpr),
            Expr::Index(index) => self.visit_index(index),
            Expr::Call(call) => self.visit_call(call),
            Expr::FieldAccess(fieldaccess) => self.visit_field_access(fieldaccess),
            Expr::Tuple(tuple) => self.visit_tuple(tuple),
            Expr::Range(range) => self.visit_range(range),
            Expr::Yield(yieldexpr) => self.visit_yield(yieldexpr),
            Expr::Try(tri) => self.visit_try(tri),
            Expr::Await(awaitexpr) => self.visit_await(awaitexpr),
            Expr::Assign(assign) => self.visit_assign(assign),
            Expr::Let(let_) => self.visit_let(let_),
            Expr::Select(sel) => self.visit_select(sel),
            Expr::Object(object) => self.visit_object(object),
            Expr::Vec(vec) => self.visit_vec(vec),
            Expr::Empty(empty) => self.visit_empty(empty),
            Expr::MacroCall(macrocall) => self.visit_macro_call(macrocall, None),
        }
    }

    fn visit_macro_call(&mut self, macrocall: &MacroCall, semi: Option<SemiColon>) -> Result<()> {
        // Note: We don't visit the stream, as emitting it truthfully is quite hard and we can't format it. Instead we just resolve everything between the open/close.
        let MacroCall {
            id: _,
            attributes,
            path,
            bang,
            open,
            stream: _,
            close,
        } = macrocall;

        let first = &path.first;

        if let PathSegment::Ident(ident) = first {
            if let LitSource::BuiltIn(BuiltIn::Template) = ident.source {
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
            self.writer.write_spanned_raw(semi.span, false, false)?;
        }

        Ok(())
    }

    fn visit_empty(&mut self, empty: &ExprEmpty) -> Result<()> {
        let ExprEmpty {
            attributes,
            open,
            expr,
            close,
        } = empty;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_vec(&mut self, vec: &ExprVec) -> Result<()> {
        let ExprVec { attributes, items } = vec;

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

    fn visit_object(&mut self, object: &ExprObject) -> Result<()> {
        let ExprObject {
            attributes,
            ident,
            assignments,
        } = object;

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

    fn visit_object_assignment(&mut self, assignment: &FieldAssign) -> Result<()> {
        let FieldAssign { key, assign } = assignment;

        match key {
            ObjectKey::LitStr(key) => {
                self.writer.write_spanned_raw(key.span, false, false)?;
            }
            ObjectKey::Path(path) => self.visit_path(path)?,
        }

        if let Some((colon, assign)) = assign {
            self.writer.write_spanned_raw(colon.span, false, true)?;
            self.visit_expr(assign)?;
        }

        Ok(())
    }

    fn visit_select(&mut self, sel: &ExprSelect) -> Result<()> {
        let ExprSelect {
            attributes,
            select,
            open,
            branches,
            close,
        } = sel;

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

    fn visit_select_branch(&mut self, branch: &ExprSelectBranch) -> Result<()> {
        match branch {
            ExprSelectBranch::Pat(pat) => self.visit_select_pattern(pat)?,
            ExprSelectBranch::Default(_default) => write!(self.writer, "default")?,
        }

        Ok(())
    }

    fn visit_select_pattern(&mut self, pat: &ExprSelectPatBranch) -> Result<()> {
        let ExprSelectPatBranch {
            pat,
            eq,
            expr,
            rocket,
            body,
        } = pat;

        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(rocket.span, false, true)?;
        self.visit_expr(body)?;

        Ok(())
    }

    fn visit_assign(&mut self, assign: &ExprAssign) -> Result<()> {
        let ExprAssign {
            attributes,
            lhs,
            eq,
            rhs,
        } = assign;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(lhs)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(rhs)?;

        Ok(())
    }

    fn visit_await(&mut self, awaitexpr: &ExprAwait) -> Result<()> {
        let ExprAwait {
            attributes,
            expr,
            dot,
            await_token,
        } = awaitexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(dot.span, false, false)?;
        self.writer
            .write_spanned_raw(await_token.span, false, false)?;

        Ok(())
    }

    fn visit_try(&mut self, tri: &ExprTry) -> Result<()> {
        let ExprTry {
            attributes,
            expr,
            try_token,
        } = tri;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer
            .write_spanned_raw(try_token.span, false, false)?;

        Ok(())
    }

    fn visit_yield(&mut self, yieldexpr: &ExprYield) -> Result<()> {
        let ExprYield {
            attributes,
            expr,
            yield_token,
        } = yieldexpr;

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

    fn visit_range(&mut self, range: &ExprRange) -> Result<()> {
        let ExprRange {
            attributes,
            from,
            limits,
            to,
        } = range;

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

    fn visit_tuple(&mut self, tuple: &ExprTuple) -> Result<()> {
        let ExprTuple { attributes, items } = tuple;

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

    fn visit_field_access(&mut self, fieldaccess: &ExprFieldAccess) -> Result<()> {
        let ExprFieldAccess {
            attributes,
            expr,
            dot,
            expr_field,
        } = fieldaccess;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(dot.span, false, false)?;
        self.visit_expr_field(expr_field)?;

        Ok(())
    }

    fn visit_expr_field(&mut self, expr_field: &ExprField) -> Result<()> {
        match expr_field {
            ExprField::Path(path) => self.visit_path(path)?,
            ExprField::LitNumber(num) => self.writer.write_spanned_raw(num.span, false, false)?,
        }

        Ok(())
    }

    fn visit_call(&mut self, call: &ExprCall) -> Result<()> {
        let ExprCall {
            id: _,
            attributes,
            expr,
            args,
        } = call;

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

    fn visit_index(&mut self, index: &ExprIndex) -> Result<()> {
        let ExprIndex {
            attributes,
            target,
            open,
            index,
            close,
        } = index;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(target)?;
        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(index)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_continue(&mut self, continueexpr: &ExprContinue) -> Result<()> {
        let ExprContinue {
            attributes,
            continue_token,
            label,
        } = continueexpr;

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

    fn visit_break(&mut self, breakexpr: &ExprBreak) -> Result<()> {
        let ExprBreak {
            attributes,
            break_token,
            expr,
        } = breakexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.writer
            .write_spanned_raw(break_token.span, false, false)?;

        if let Some(expr) = expr {
            self.writer.write_unspanned(" ")?;
            self.visit_expr_break_value(expr)?;
        }

        Ok(())
    }

    fn visit_expr_break_value(&mut self, breakvalue: &ExprBreakValue) -> Result<()> {
        match breakvalue {
            ExprBreakValue::Expr(expr) => self.visit_expr(expr)?,
            ExprBreakValue::Label(label) => {
                self.writer.write_spanned_raw(label.span, false, false)?
            }
        }

        Ok(())
    }

    fn visit_return(&mut self, returnexpr: &ExprReturn) -> Result<()> {
        let ExprReturn {
            attributes,
            return_token,
            expr,
        } = returnexpr;

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

    fn visit_closure(&mut self, closure: &ExprClosure) -> Result<()> {
        let ExprClosure {
            id: _,
            attributes,
            async_token,
            move_token,
            args,
            body,
        } = closure;

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
            ExprClosureArgs::Empty { token } => {
                self.writer.write_spanned_raw(token.span, false, true)?
            }
            ExprClosureArgs::List { args, open, close } => {
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

    fn visit_match(&mut self, matchexpr: &ExprMatch) -> Result<()> {
        let ExprMatch {
            attributes,
            match_,
            expr,
            open,
            branches,
            close,
        } = matchexpr;

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

    fn visit_match_branch(&mut self, branch: &ExprMatchBranch) -> Result<bool> {
        let ExprMatchBranch {
            pat,
            condition,
            rocket,
            body,
        } = branch;

        self.visit_pattern(pat)?;

        if let Some((if_, expr)) = condition {
            self.writer.write_unspanned(" ")?;
            self.writer.write_spanned_raw(if_.span, false, true)?;
            self.visit_expr(expr)?;
        }
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(rocket.span, false, true)?;
        self.visit_expr(body)?;

        let should_have_comma = !matches!(body, Expr::Block(_));

        Ok(should_have_comma)
    }

    fn visit_loop(&mut self, loopexpr: &ExprLoop) -> Result<()> {
        let ExprLoop {
            attributes,
            label,
            loop_token,
            body,
        } = loopexpr;

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

    fn visit_for(&mut self, forexpr: &ExprFor) -> Result<()> {
        let ExprFor {
            attributes,
            label,
            binding,
            in_,
            iter,
            body,
            for_token,
        } = forexpr;

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

    fn visit_while(&mut self, whileexpr: &ExprWhile) -> Result<()> {
        let ExprWhile {
            attributes,
            label,
            while_token,
            condition,
            body,
        } = whileexpr;

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

    fn visit_condition(&mut self, condition: &Condition) -> Result<()> {
        match condition {
            Condition::Expr(expr) => self.visit_expr(expr),
            Condition::ExprLet(let_) => self.visit_let(let_),
        }
    }

    fn visit_pattern(&mut self, pattern: &Pat) -> Result<()> {
        match pattern {
            Pat::PatIgnore(ignore) => self.visit_pat_ignore(ignore)?,
            Pat::PatPath(path) => self.visit_pat_path(path)?,
            Pat::PatLit(patit) => self.visit_pat_lit(patit)?,
            Pat::PatVec(patvec) => self.visit_pat_vec(patvec)?,
            Pat::PatTuple(pattuple) => self.visit_pat_tuple(pattuple)?,
            Pat::PatObject(patobject) => self.visit_pat_object(patobject)?,
            Pat::PatBinding(binding) => self.visit_pat_binding(binding)?,
            Pat::PatRest(rest) => self.visit_pat_rest(rest)?,
        }

        Ok(())
    }

    fn visit_pat_rest(&mut self, rest: &PatRest) -> Result<()> {
        let PatRest {
            attributes,
            dot_dot,
        } = rest;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(dot_dot.span, false, false)?;

        Ok(())
    }

    fn visit_pat_binding(&mut self, binding: &PatBinding) -> Result<()> {
        let PatBinding {
            attributes,
            key,
            colon,
            pat,
        } = binding;

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

    fn visit_pat_object(&mut self, patobject: &PatObject) -> Result<()> {
        let PatObject {
            attributes,
            ident,
            items,
        } = patobject;

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

        let Braced {
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

    fn visit_pat_tuple(&mut self, pattuple: &PatTuple) -> Result<()> {
        let PatTuple {
            attributes,
            items,
            path,
        } = pattuple;

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

    fn visit_pat_vec(&mut self, patvec: &PatVec) -> Result<()> {
        let PatVec { attributes, items } = patvec;

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

    fn visit_pat_lit(&mut self, patit: &PatLit) -> Result<()> {
        let PatLit { attributes, expr } = patit;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_expr(expr)?;

        Ok(())
    }

    fn visit_pat_ignore(&mut self, ignore: &PatIgnore) -> Result<()> {
        let PatIgnore {
            attributes,
            underscore,
        } = ignore;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer
            .write_spanned_raw(underscore.span, false, false)?;

        Ok(())
    }

    fn visit_pat_path(&mut self, path: &PatPath) -> Result<()> {
        let PatPath { attributes, path } = path;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }
        self.visit_path(path)?;

        Ok(())
    }
    fn visit_let(&mut self, let_: &ExprLet) -> Result<()> {
        let ExprLet {
            attributes,
            let_token,
            pat,
            eq,
            expr,
        } = let_;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(let_token.span, false, true)?;
        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        Ok(())
    }
    fn visit_if(&mut self, ifexpr: &ExprIf) -> Result<()> {
        let ExprIf {
            attributes,
            if_,
            condition,
            block,
            expr_else_ifs,
            expr_else,
        } = ifexpr;

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

    fn visit_expr_else_if(&mut self, expr_else_if: &ExprElseIf) -> Result<()> {
        let ExprElseIf {
            else_,
            if_,
            condition,
            block,
        } = expr_else_if;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(else_.span, false, true)?;
        self.writer.write_spanned_raw(if_.span, false, true)?;

        self.visit_condition(condition)?;
        write!(self.writer, " ")?;
        self.visit_block(block)?;

        Ok(())
    }

    fn visit_expr_else(&mut self, expr_else: &ExprElse) -> Result<()> {
        let ExprElse { else_, block } = expr_else;

        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(else_.span, false, true)?;

        self.visit_block(block)?;
        Ok(())
    }

    fn visit_expr_block(&mut self, block: &ExprBlock) -> Result<()> {
        let ExprBlock {
            attributes,
            async_token,
            const_token,
            move_token,
            block,
        } = block;

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

    fn visit_block(&mut self, block: &Block) -> Result<()> {
        let Block {
            id: _,
            open,
            statements,
            close,
        } = block;

        self.writer.write_spanned_raw(open.span, true, false)?;

        self.writer.indent();
        for statement in statements {
            self.visit_statement(statement)?;
        }

        self.writer.dedent();
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<()> {
        match statement {
            Stmt::Local(local) => {
                self.visit_local(local)?;
                self.writer.newline()?;
            }
            Stmt::Item(item, semi) => {
                self.visit_item(item, *semi)?;
                if !matches!(item, Item::Fn(_)) {
                    self.writer.newline()?;
                }
            }
            Stmt::Expr(expr) => {
                self.visit_expr(expr)?;
                self.writer.newline()?;
            }
            Stmt::Semi(semi) => {
                let StmtSemi { expr, semi_token } = semi;

                self.visit_expr(expr)?;
                self.writer
                    .write_spanned_raw(semi_token.span, false, false)?;
                self.writer.newline()?;
            }
        }

        Ok(())
    }

    fn visit_local(&mut self, local: &Local) -> Result<()> {
        let Local {
            attributes,
            let_token,
            pat,
            eq,
            expr,
            semi,
        } = local;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(let_token.span, false, true)?;
        self.visit_pattern(pat)?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(eq.span, false, true)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(semi.span, false, false)?;

        Ok(())
    }

    fn visit_unary(&mut self, unary: &ExprUnary) -> Result<()> {
        let ExprUnary {
            op,
            expr,
            attributes,
        } = unary;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(op.span(), false, false)?;

        self.visit_expr(expr)
    }

    fn visit_group(&mut self, group: &ExprGroup) -> Result<()> {
        let ExprGroup {
            attributes,
            open,
            expr,
            close,
        } = group;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.writer.write_spanned_raw(open.span, false, false)?;
        self.visit_expr(expr)?;
        self.writer.write_spanned_raw(close.span, false, false)?;

        Ok(())
    }

    fn visit_path(&mut self, path: &Path) -> Result<()> {
        let Path {
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

    fn visit_lit(&mut self, lit: &ExprLit) -> Result<()> {
        let ExprLit { attributes, lit } = lit;

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

    fn visit_binary(&mut self, binary: &ExprBinary) -> Result<()> {
        let ExprBinary {
            attributes,
            op,
            lhs,
            rhs,
        } = binary;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_expr(lhs.as_ref())?;
        self.writer.write_unspanned(" ")?;
        self.writer.write_spanned_raw(op.span(), false, true)?;
        self.visit_expr(rhs.as_ref())?;
        Ok(())
    }

    fn visit_path_segment_expr(&mut self, expr: &PathSegmentExpr) -> Result<()> {
        let PathSegmentExpr { expr } = expr;
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
                let Braced {
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

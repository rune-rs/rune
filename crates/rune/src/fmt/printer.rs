// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2023, Tom Solberg, all rights reserved.
// Created: 27 April 2023

/*!
 * The `Printer` trait and implementations.
 */

use std::io::Write;

use super::error::FormattingError;
use super::indent_writer::IndentedWriter;
use crate::{
    ast::{
        AngleBracketed, AttrStyle, Block, Braced, BuiltIn, Comma, Condition, Else, Expr,
        ExprAssign, ExprAwait, ExprBinary, ExprBlock, ExprBreak, ExprBreakValue, ExprCall,
        ExprClosure, ExprClosureArgs, ExprContinue, ExprElse, ExprElseIf, ExprEmpty, ExprField,
        ExprFieldAccess, ExprFor, ExprGroup, ExprIf, ExprIndex, ExprLet, ExprLit, ExprLoop,
        ExprMatch, ExprMatchBranch, ExprObject, ExprRange, ExprReturn, ExprSelect,
        ExprSelectBranch, ExprSelectPatBranch, ExprTry, ExprTuple, ExprUnary, ExprVec, ExprWhile,
        ExprYield, Field, FieldAssign, FnArg, ItemConst, ItemEnum, ItemFn, ItemImpl, ItemMod,
        ItemModBody, ItemStruct, ItemStructBody, ItemVariant, ItemVariantBody, LitSource, Local,
        MacroCall, ObjectKey, Pat, PatBinding, PatIgnore, PatLit, PatObject, PatPath, PatRest,
        PatTuple, PatVec, Path, PathSegment, PathSegmentExpr, SelfType, SelfValue, SemiColon, Span,
        Stmt, StmtSemi,
    },
    compile::attrs::Attributes,
    Source,
};

pub struct Printer<'a, W>
where
    W: Write,
{
    writer: IndentedWriter<W>,
    source: &'a Source,
}

impl<'a, W> Printer<'a, W>
where
    W: Write,
{
    pub fn new(writer: W, source: &'a Source) -> Self {
        Self {
            writer: IndentedWriter::new(writer),
            source,
        }
    }

    pub fn resolve(&self, span: Span) -> Result<String, FormattingError> {
        match self.source.get(span.range()) {
            Some(s) => Ok(s.to_owned()),
            None => Err(FormattingError::InvalidSpan(
                span.start.into_usize(),
                span.end.into_usize(),
                self.source.len(),
            )),
        }
    }

    pub fn visit_file(&mut self, file: &crate::ast::File) -> Result<(), FormattingError> {
        if let Some(shebang) = &file.shebang {
            writeln!(self.writer, "{}", self.resolve(shebang.span)?)?;
        }

        for attribute in &file.attributes {
            self.visit_attribute(attribute);
            writeln!(self.writer);
        }

        for item in &file.items {
            self.visit_item(&item.0, item.1)?;
        }

        Ok(())
    }

    pub fn visit_attribute(
        &mut self,
        attribute: &crate::ast::Attribute,
    ) -> Result<(), FormattingError> {
        let crate::ast::Attribute {
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
                write!(self.writer, "{}", self.resolve(ident.span)?.trim())?;

                return Ok(());
            }
        }

        write!(self.writer, "{}", self.resolve(hash.span)?)?;
        match style {
            AttrStyle::Outer(_) => write!(self.writer, "!")?,
            AttrStyle::Inner => write!(self.writer, "")?,
        }

        write!(self.writer, "{}", self.resolve(open.span)?)?;
        self.visit_path(path)?;
        for token in input {
            write!(self.writer, "{}", self.resolve(token.span)?)?;
        }
        write!(self.writer, "{}", self.resolve(close.span)?)?;

        Ok(())
    }

    pub fn visit_item(
        &mut self,
        item: &crate::ast::Item,
        semi: Option<SemiColon>,
    ) -> Result<(), FormattingError> {
        match item {
            crate::ast::Item::Use(usage) => self.visit_use(usage),
            crate::ast::Item::Fn(item) => self.visit_fn(item),
            crate::ast::Item::Enum(item) => self.visit_enum(item),
            crate::ast::Item::Struct(item) => self.visit_struct(item),
            crate::ast::Item::Impl(item) => self.visit_impl(item),
            crate::ast::Item::Mod(item) => self.visit_mod(item),
            crate::ast::Item::Const(item) => self.visit_const(item),
            crate::ast::Item::MacroCall(item) => self.visit_macro_call(item),
        }
    }

    fn visit_const(&mut self, item: &ItemConst) -> Result<(), FormattingError> {
        let ItemConst {
            id,
            attributes,
            visibility,
            const_token,
            name,
            eq,
            expr,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute);
        }
        writeln!(self.writer)?;

        self.emit_visibility(visibility)?;
        write!(self.writer, "const ")?;
        write!(self.writer, "{}", self.resolve(name.span)?)?;
        write!(self.writer, " = ")?;
        self.visit_expr(expr)?;

        Ok(())
    }

    fn visit_mod(&mut self, item: &ItemMod) -> Result<(), FormattingError> {
        let ItemMod {
            id,
            attributes,
            visibility,
            mod_token,
            name,
            body,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute);
        }
        writeln!(self.writer)?;

        self.emit_visibility(visibility)?;

        write!(self.writer, "mod ")?;
        write!(self.writer, "{}", self.resolve(name.span)?)?;

        match body {
            ItemModBody::EmptyBody(semi) => {
                writeln!(self.writer, ";")?;
            }
            ItemModBody::InlineBody(body) => {
                writeln!(self.writer, " {{")?;
                self.writer.indent();

                self.visit_file(&body.file)?;

                self.writer.dedent();
                write!(self.writer, "}}")?;
            }
        }

        Ok(())
    }
    fn visit_impl(&mut self, item: &ItemImpl) -> Result<(), FormattingError> {
        let ItemImpl {
            attributes,
            impl_,
            path,
            open,
            functions,
            close,
        } = item;

        for attribute in attributes {
            self.visit_attribute(attribute);
        }
        writeln!(self.writer)?;

        write!(self.writer, "impl ")?;
        self.visit_path(path)?;

        write!(self.writer, " {{")?;
        self.writer.indent();

        for function in functions {
            self.visit_fn(function)?;
            writeln!(self.writer)?;
        }

        self.writer.dedent();
        write!(self.writer, "}}")?;

        Ok(())
    }
    fn visit_struct(&mut self, item: &ItemStruct) -> Result<(), FormattingError> {
        let ItemStruct {
            id,
            attributes,
            visibility,
            struct_token,
            ident,
            body,
        } = item;

        for attribute in &item.attributes {
            self.visit_attribute(attribute);
        }
        writeln!(self.writer)?;

        self.emit_visibility(visibility)?;
        write!(self.writer, "struct ")?;

        write!(self.writer, "{} ", self.resolve(ident.span)?)?;

        Ok(())
    }

    fn visit_struct_body(&mut self, body: &ItemStructBody) -> Result<(), FormattingError> {
        match body {
            ItemStructBody::UnitBody => {}
            ItemStructBody::TupleBody(tuple) => {
                write!(self.writer, "(")?;
                for (field, comma) in tuple {
                    self.visit_field(field)?;
                    if comma.is_some() {
                        write!(self.writer, ",")?;
                    }
                }
                write!(self.writer, ")")?;
            }
            ItemStructBody::StructBody(body) => {
                write!(self.writer, "{{")?;
                self.writer.indent();
                for (field, comma) in body {
                    self.visit_field(field)?;
                    if comma.is_some() {
                        write!(self.writer, ",")?;
                    }
                }
                self.writer.dedent();
                write!(self.writer, "}}")?;
            }
        }

        Ok(())
    }

    fn visit_enum(&mut self, item: &ItemEnum) -> Result<(), FormattingError> {
        let ItemEnum {
            attributes,
            visibility,
            enum_token,
            name,
            variants,
        } = item;

        for attribute in &item.attributes {
            self.visit_attribute(attribute);
        }
        writeln!(self.writer)?;

        self.emit_visibility(visibility)?;
        write!(self.writer, "enum ")?;

        write!(self.writer, "{}", self.resolve(name.span)?)?;
        write!(self.writer, " ")?;

        write!(self.writer, "{{")?;
        self.writer.indent();
        for (variant, comma) in variants {
            self.visit_variant(variant)?;
            if comma.is_some() {
                write!(self.writer, ",")?;
            }
        }
        self.writer.dedent();
        write!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_variant(&mut self, variant: &ItemVariant) -> Result<(), FormattingError> {
        let ItemVariant {
            id,
            attributes,
            name,
            body,
        } = variant;

        for attribute in &variant.attributes {
            self.visit_attribute(attribute);
        }

        write!(self.writer, "{}", self.resolve(name.span)?)?;

        self.visit_variant_body(body)?;

        Ok(())
    }

    fn visit_variant_body(&mut self, body: &ItemVariantBody) -> Result<(), FormattingError> {
        match body {
            ItemVariantBody::UnitBody => {}
            ItemVariantBody::TupleBody(body) => {
                write!(self.writer, "(")?;
                for (field, comma) in &body.parenthesized {
                    self.visit_field(field);
                    if let Some(comma) = comma {
                        write!(self.writer, ",")?;
                    }
                }
                write!(self.writer, ")")?;
            }
            ItemVariantBody::StructBody(sbody) => {
                write!(self.writer, "{{")?;
                self.writer.indent();
                for (field, comma) in &sbody.braced {
                    self.visit_field(field)?;
                    if let Some(comma) = comma {
                        write!(self.writer, ",")?;
                    }
                }
                self.writer.dedent();
                write!(self.writer, "}}")?;
            }
        }

        Ok(())
    }

    fn visit_field(&mut self, field: &Field) -> Result<(), FormattingError> {
        let Field {
            attributes,
            visibility,
            name,
        } = field;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.emit_visibility(visibility)?;
        write!(self.writer, "{}", self.resolve(name.span)?)?;

        Ok(())
    }

    fn visit_fn(&mut self, item: &ItemFn) -> Result<(), FormattingError> {
        let ItemFn {
            id,
            attributes,
            visibility,
            const_token,
            async_token,
            fn_token,
            name,
            args,
            body,
        } = item;

        dbg!(attributes.len());
        for attribute in attributes {
            self.visit_attribute(attribute)?;
            write!(self.writer, "\n").unwrap();
        }

        self.emit_visibility(visibility)?;
        if const_token.is_some() {
            write!(self.writer, "const ")?;
        }

        if async_token.is_some() {
            write!(self.writer, "async ")?;
        }
        write!(self.writer, "fn ")?;
        write!(self.writer, "{}", self.resolve(name.span)?)?;

        write!(self.writer, "(")?;
        if args.len() > 5 {
            self.writer.indent();
            writeln!(self.writer)?;
        }
        for (arg, comma) in args {
            match arg {
                FnArg::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
                FnArg::Pat(pattern) => self.visit_pattern(pattern)?,
            }
            if let Some(comma) = comma {
                write!(self.writer, ",")?;
            }
        }
        if args.len() > 5 {
            self.writer.dedent();
            writeln!(self.writer)?;
        }
        write!(self.writer, ") ")?;
        self.visit_block(body)?;
        write!(self.writer, "\n")?;

        Ok(())
    }

    fn visit_use(&mut self, usage: &crate::ast::ItemUse) -> Result<(), FormattingError> {
        let crate::ast::ItemUse {
            attributes,
            visibility,
            use_token,
            path,
        } = usage;
        for attribute in &usage.attributes {
            self.visit_attribute(attribute);
        }

        self.emit_visibility(visibility)?;
        write!(self.writer, "use ")?;
        self.visit_item_use_path(path, None)
    }

    fn visit_item_use_path(
        &mut self,
        path: &crate::ast::ItemUsePath,
        comma: Option<Comma>,
    ) -> Result<(), FormattingError> {
        let crate::ast::ItemUsePath {
            global,
            first,
            segments,
            alias,
        } = path;

        if global.is_some() {
            write!(self.writer, "::")?;
        }

        self.visit_item_use_segment(first)?;
        for (_, segment) in segments {
            write!(self.writer, "::")?;
            self.visit_item_use_segment(segment)?;
        }

        if let Some(alias) = alias {
            write!(self.writer, " as {}", self.resolve(alias.1.span)?)?;
        }

        if comma.is_some() {
            write!(self.writer, ", ")?;
        }

        Ok(())
    }

    fn visit_path_segment(
        &mut self,
        segment: &crate::ast::PathSegment,
    ) -> Result<(), FormattingError> {
        match segment {
            PathSegment::SelfType(selftype) => self.visit_self_type(selftype)?,
            PathSegment::SelfValue(selfvalue) => self.visit_self_value(selfvalue)?,
            PathSegment::Ident(ident) => write!(self.writer, "{}", self.resolve(ident.span)?)?,
            PathSegment::Crate(c) => write!(self.writer, "{}", self.resolve(c.span)?)?,
            PathSegment::Super(s) => write!(self.writer, "{}", self.resolve(s.span)?)?,
            PathSegment::Generics(g) => self.visit_generics(g)?,
        }
        Ok(())
    }

    fn visit_self_type(&mut self, selftype: &SelfType) -> Result<(), FormattingError> {
        write!(self.writer, "Self")?;
        Ok(())
    }

    fn visit_self_value(&mut self, selfvalue: &SelfValue) -> Result<(), FormattingError> {
        write!(self.writer, "self")?;
        Ok(())
    }

    fn visit_generics(
        &mut self,
        generics: &AngleBracketed<PathSegmentExpr, Comma>,
    ) -> Result<(), FormattingError> {
        write!(self.writer, "<")?;

        for (expr, comma) in generics {
            self.visit_path_segment_expr(expr)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }

        write!(self.writer, ">")?;

        Ok(())
    }

    fn visit_expr(&mut self, expr: &Expr) -> Result<(), FormattingError> {
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
            Expr::MacroCall(macrocall) => self.visit_macro_call(macrocall),
        }
    }

    fn visit_macro_call(&mut self, macrocall: &MacroCall) -> Result<(), FormattingError> {
        let MacroCall {
            id,
            attributes,
            path,
            bang,
            open,
            stream,
            close,
        } = macrocall;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_path(path)?;

        write!(self.writer, "!")?;
        write!(self.writer, "{}", self.resolve(open.span)?)?;

        for token in stream {
            write!(self.writer, "{}", self.resolve(token.span)?)?;
        }

        write!(self.writer, "{}", self.resolve(close.span)?)?;

        Ok(())
    }

    fn visit_empty(&mut self, empty: &ExprEmpty) -> Result<(), FormattingError> {
        write!(self.writer, "()")?;
        Ok(())
    }

    fn visit_vec(&mut self, vec: &ExprVec) -> Result<(), FormattingError> {
        let ExprVec { attributes, items } = vec;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if items.len() > 5 {
            write!(self.writer, "[")?;
            self.writer.indent();
            writeln!(self.writer)?;
        } else {
            write!(self.writer, "[ ")?;
        }

        for (item, comma) in items {
            self.visit_expr(item)?;

            write!(self.writer, ", ")?;
        }

        if items.len() > 5 {
            self.writer.dedent();
            writeln!(self.writer)?;
            write!(self.writer, "]")?;
        } else {
            write!(self.writer, "]")?;
        }

        Ok(())
    }

    fn visit_object(&mut self, object: &ExprObject) -> Result<(), FormattingError> {
        let ExprObject {
            attributes,
            ident,
            assignments,
        } = object;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        match ident {
            crate::ast::ObjectIdent::Anonymous(anonymous) => write!(self.writer, "#{{")?,
            crate::ast::ObjectIdent::Named(named) => {
                self.visit_path(named)?;
                write!(self.writer, " {{")?
            }
        }

        if assignments.len() > 5 {
            self.writer.indent();
            writeln!(self.writer)?;
        }
        for (assignment, comma) in assignments {
            self.visit_object_assignment(assignment)?;

            write!(self.writer, ", ")?;
        }

        if assignments.len() > 5 {
            self.writer.dedent();
            writeln!(self.writer)?;
        }
        write!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_object_assignment(&mut self, assignment: &FieldAssign) -> Result<(), FormattingError> {
        let FieldAssign { key, assign } = assignment;

        match key {
            ObjectKey::LitStr(key) => write!(self.writer, "{}", self.resolve(key.span)?)?,
            ObjectKey::Path(path) => self.visit_path(path)?,
        }

        if let Some((colon, assign)) = assign {
            write!(self.writer, ": ")?;
            self.visit_expr(assign)?;
        }

        Ok(())
    }

    fn visit_select(&mut self, sel: &ExprSelect) -> Result<(), FormattingError> {
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

        write!(self.writer, "select {{")?;
        self.writer.indent();
        for (branch, comma) in branches {
            self.visit_select_branch(branch)?;
            writeln!(self.writer, ", ")?;
        }
        self.writer.dedent();
        write!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_select_branch(&mut self, branch: &ExprSelectBranch) -> Result<(), FormattingError> {
        match branch {
            ExprSelectBranch::Pat(pat) => self.visit_select_pattern(pat)?,
            ExprSelectBranch::Default(default) => write!(self.writer, "default")?,
        }

        Ok(())
    }

    fn visit_select_pattern(&mut self, pat: &ExprSelectPatBranch) -> Result<(), FormattingError> {
        let ExprSelectPatBranch {
            pat,
            eq,
            expr,
            rocket,
            body,
        } = pat;

        self.visit_pattern(pat)?;
        write!(self.writer, " = ")?;
        self.visit_expr(expr)?;
        write!(self.writer, " => ")?;
        self.visit_expr(body)?;

        Ok(())
    }

    fn visit_assign(&mut self, assign: &ExprAssign) -> Result<(), FormattingError> {
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
        write!(self.writer, " = ")?;
        self.visit_expr(rhs)?;

        Ok(())
    }

    fn visit_await(&mut self, awaitexpr: &ExprAwait) -> Result<(), FormattingError> {
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
        write!(self.writer, ".")?;
        write!(self.writer, "await ")?;

        Ok(())
    }

    fn visit_try(&mut self, tri: &ExprTry) -> Result<(), FormattingError> {
        let ExprTry {
            attributes,
            expr,
            try_token,
        } = tri;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        write!(self.writer, " ?")?;

        Ok(())
    }

    fn visit_yield(&mut self, yieldexpr: &ExprYield) -> Result<(), FormattingError> {
        let ExprYield {
            attributes,
            expr,
            yield_token,
        } = yieldexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        write!(self.writer, "yield")?;

        if let Some(expr) = expr {
            write!(self.writer, " ")?;
            self.visit_expr(expr)?;
        }

        Ok(())
    }

    fn visit_range(&mut self, range: &ExprRange) -> Result<(), FormattingError> {
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
            crate::ast::ExprRangeLimits::HalfOpen(_) => write!(self.writer, "..")?,
            crate::ast::ExprRangeLimits::Closed(_) => write!(self.writer, "..=")?,
        }

        if let Some(to) = to {
            self.visit_expr(to)?;
        }

        Ok(())
    }

    fn visit_tuple(&mut self, tuple: &ExprTuple) -> Result<(), FormattingError> {
        let ExprTuple { attributes, items } = tuple;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        write!(self.writer, "(")?;
        for (item, comma) in items {
            self.visit_expr(item)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }

        Ok(())
    }

    fn visit_field_access(&mut self, fieldaccess: &ExprFieldAccess) -> Result<(), FormattingError> {
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
        write!(self.writer, ".")?;
        self.visit_expr_field(expr_field)?;

        Ok(())
    }

    fn visit_expr_field(&mut self, expr_field: &ExprField) -> Result<(), FormattingError> {
        match expr_field {
            ExprField::Path(path) => self.visit_path(path)?,
            ExprField::LitNumber(num) => write!(self.writer, "{}", self.resolve(num.span)?)?,
        }

        Ok(())
    }

    fn visit_call(&mut self, call: &ExprCall) -> Result<(), FormattingError> {
        let ExprCall {
            id,
            attributes,
            expr,
            args,
        } = call;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        self.visit_expr(expr)?;
        write!(self.writer, "(")?;

        for (arg, comma) in &args.parenthesized {
            self.visit_expr(arg)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }
        write!(self.writer, ")")?;

        Ok(())
    }

    fn visit_index(&mut self, index: &ExprIndex) -> Result<(), FormattingError> {
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
        write!(self.writer, "[")?;
        self.visit_expr(index)?;
        write!(self.writer, "]")?;

        Ok(())
    }

    fn visit_continue(&mut self, continueexpr: &ExprContinue) -> Result<(), FormattingError> {
        let ExprContinue {
            attributes,
            continue_token,
            label,
        } = continueexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        write!(self.writer, "continue")?;

        if let Some(label) = label {
            write!(self.writer, " ")?;
            write!(self.writer, "{}", self.resolve(label.span)?)?;
        }

        Ok(())
    }

    fn visit_break(&mut self, breakexpr: &ExprBreak) -> Result<(), FormattingError> {
        let ExprBreak {
            attributes,
            break_token,
            expr,
        } = breakexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        write!(self.writer, "break")?;

        if let Some(expr) = expr {
            write!(self.writer, " ")?;
            self.visit_expr_break_value(expr)?;
        }

        Ok(())
    }

    fn visit_expr_break_value(
        &mut self,
        breakvalue: &ExprBreakValue,
    ) -> Result<(), FormattingError> {
        match breakvalue {
            ExprBreakValue::Expr(expr) => self.visit_expr(expr)?,
            ExprBreakValue::Label(label) => write!(self.writer, "{}", self.resolve(label.span)?)?,
        }

        Ok(())
    }

    fn visit_return(&mut self, returnexpr: &ExprReturn) -> Result<(), FormattingError> {
        let ExprReturn {
            attributes,
            return_token,
            expr,
        } = returnexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        write!(self.writer, "return")?;

        if let Some(expr) = expr {
            write!(self.writer, " ")?;
            self.visit_expr(expr)?;
        }

        Ok(())
    }

    fn visit_closure(&mut self, closure: &ExprClosure) -> Result<(), FormattingError> {
        let ExprClosure {
            id,
            attributes,
            async_token,
            move_token,
            args,
            body,
        } = closure;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if async_token.is_some() {
            write!(self.writer, " async")?;
        }

        if move_token.is_some() {
            write!(self.writer, " move")?;
        }

        match args {
            ExprClosureArgs::Empty { .. } => write!(self.writer, "|| ")?,
            ExprClosureArgs::List { args, .. } => {
                write!(self.writer, "|")?;
                for (arg, comma) in args {
                    match arg {
                        crate::ast::FnArg::SelfValue(self_) => self.visit_self_value(self_)?,
                        crate::ast::FnArg::Pat(pat) => self.visit_pattern(pat)?,
                    }
                    if comma.is_some() {
                        write!(self.writer, ", ")?;
                    }
                }
                write!(self.writer, "| ")?;
            }
        }

        self.visit_expr(body)?;

        Ok(())
    }

    fn visit_match(&mut self, matchexpr: &ExprMatch) -> Result<(), FormattingError> {
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

        write!(self.writer, "{} ", self.resolve(match_.span)?)?;
        self.visit_expr(expr)?;
        writeln!(self.writer, " {{")?;

        self.writer.indent();
        for (branch, comma) in branches {
            self.visit_match_branch(branch)?;
            if comma.is_some() {
                writeln!(self.writer, ",")?;
            }
        }
        self.writer.dedent();

        write!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_match_branch(&mut self, branch: &ExprMatchBranch) -> Result<(), FormattingError> {
        let ExprMatchBranch {
            pat,
            condition,
            rocket,
            body,
        } = branch;

        self.visit_pattern(pat)?;

        if let Some((if_, expr)) = condition {
            write!(self.writer, " {} ", self.resolve(if_.span)?)?;
            self.visit_expr(expr)?;
        }

        write!(self.writer, " {} ", self.resolve(rocket.span)?)?;
        self.visit_expr(body)?;

        Ok(())
    }
    fn visit_loop(&mut self, loopexpr: &ExprLoop) -> Result<(), FormattingError> {
        let ExprLoop {
            attributes,
            label,
            loop_token,
            body,
        } = loopexpr;

        for attr in attributes {
            self.visit_attribute(attr)?;
        }

        if let Some(label) = label {
            write!(self.writer, "{}: ", self.resolve(label.0.span)?)?;
        }

        write!(self.writer, "{} ", self.resolve(loop_token.span)?)?;

        self.visit_block(body)?;
        writeln!(self.writer)?;

        Ok(())
    }

    fn visit_for(&mut self, forexpr: &ExprFor) -> Result<(), FormattingError> {
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

        if let Some(label) = label {
            write!(self.writer, "{}: ", self.resolve(label.0.span)?)?;
        }

        write!(self.writer, "{} ", self.resolve(for_token.span)?)?;

        self.visit_pattern(binding)?;

        write!(self.writer, " {} ", self.resolve(in_.span)?)?;

        self.visit_expr(iter)?;

        self.visit_block(body)?;
        writeln!(self.writer)?;

        Ok(())
    }

    fn visit_while(&mut self, whileexpr: &ExprWhile) -> Result<(), FormattingError> {
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

        if let Some(label) = label {
            write!(self.writer, "{}: ", self.resolve(label.0.span)?)?;
        }

        write!(self.writer, "{} ", self.resolve(while_token.span)?)?;

        self.visit_condition(condition)?;

        self.visit_block(body)?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn visit_condition(&mut self, condition: &Condition) -> Result<(), FormattingError> {
        match condition {
            Condition::Expr(expr) => self.visit_expr(expr),
            Condition::ExprLet(let_) => self.visit_let(let_),
        }
    }

    fn visit_pattern(&mut self, pattern: &Pat) -> Result<(), FormattingError> {
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

    fn visit_pat_rest(&mut self, rest: &PatRest) -> Result<(), FormattingError> {
        let PatRest {
            attributes,
            dot_dot: _dot_dot,
        } = rest;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        write!(self.writer, "..")?;

        Ok(())
    }

    fn visit_pat_binding(&mut self, binding: &PatBinding) -> Result<(), FormattingError> {
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
            crate::ast::ObjectKey::LitStr(str_) => {
                write!(self.writer, "{}", self.resolve(str_.span)?)?
            }
            crate::ast::ObjectKey::Path(path) => self.visit_path(path)?,
        }

        write!(self.writer, ": ")?;

        self.visit_pattern(pat)?;

        Ok(())
    }

    fn visit_pat_object(&mut self, patobject: &PatObject) -> Result<(), FormattingError> {
        let PatObject {
            attributes,
            ident,
            items,
        } = patobject;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        match ident {
            crate::ast::ObjectIdent::Anonymous(v) => write!(self.writer, "#")?,
            crate::ast::ObjectIdent::Named(n) => {
                self.visit_path(n)?;
                write!(self.writer, " ")?;
            }
        }

        write!(self.writer, "{{")?;
        if items.len() > 5 {
            write!(self.writer, "\n")?;
            self.writer.indent();
        }
        for (pat, comma) in items {
            self.visit_pattern(pat)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }
        if items.len() > 5 {
            self.writer.dedent();
            write!(self.writer, "\n")?;
        }
        write!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_pat_tuple(&mut self, pattuple: &PatTuple) -> Result<(), FormattingError> {
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
        write!(self.writer, "(")?;

        for (pat, comma) in items {
            self.visit_pattern(pat)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }

        write!(self.writer, ")")?;

        Ok(())
    }

    fn visit_pat_vec(&mut self, patvec: &PatVec) -> Result<(), FormattingError> {
        let PatVec { attributes, items } = patvec;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        write!(self.writer, "[")?;

        for (pat, comma) in items {
            self.visit_pattern(pat)?;
            if comma.is_some() {
                write!(self.writer, ", ")?;
            }
        }

        write!(self.writer, "]")?;

        Ok(())
    }

    fn visit_pat_lit(&mut self, patit: &PatLit) -> Result<(), FormattingError> {
        let PatLit { attributes, expr } = patit;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        self.visit_expr(expr)?;

        Ok(())
    }

    fn visit_pat_ignore(&mut self, ignore: &PatIgnore) -> Result<(), FormattingError> {
        let PatIgnore { attributes, .. } = ignore;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        write!(self.writer, "_")?;

        Ok(())
    }

    fn visit_pat_path(&mut self, path: &PatPath) -> Result<(), FormattingError> {
        let PatPath { attributes, path } = path;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }
        self.visit_path(path)?;

        Ok(())
    }
    fn visit_let(&mut self, let_: &ExprLet) -> Result<(), FormattingError> {
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

        write!(self.writer, "let ")?;
        self.visit_pattern(pat)?;
        write!(self.writer, " = ")?;
        self.visit_expr(expr)?;
        Ok(())
    }
    fn visit_if(&mut self, ifexpr: &ExprIf) -> Result<(), FormattingError> {
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

        write!(self.writer, "if ")?;
        self.visit_condition(condition)?;
        write!(self.writer, " ")?;
        self.visit_block(block)?;

        for expr_else_if in expr_else_ifs {
            self.visit_expr_else_if(expr_else_if)?;
        }

        if let Some(expr_else) = expr_else {
            self.visit_expr_else(expr_else)?;
        }

        Ok(())
    }

    fn visit_expr_else_if(&mut self, expr_else_if: &ExprElseIf) -> Result<(), FormattingError> {
        let ExprElseIf {
            else_,
            if_,
            condition,
            block,
        } = expr_else_if;

        write!(self.writer, "else if ")?;
        self.visit_condition(condition)?;
        write!(self.writer, " ")?;
        self.visit_block(block)?;

        Ok(())
    }

    fn visit_expr_else(&mut self, expr_else: &ExprElse) -> Result<(), FormattingError> {
        let ExprElse { else_, block } = expr_else;

        write!(self.writer, "else ")?;
        self.visit_block(block)?;
        Ok(())
    }

    fn visit_expr_block(&mut self, block: &ExprBlock) -> Result<(), FormattingError> {
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

        if async_token.is_some() {
            write!(self.writer, "async ")?;
        }

        if const_token.is_some() {
            write!(self.writer, "const ")?;
        }

        if move_token.is_some() {
            write!(self.writer, "move ")?;
        }

        self.visit_block(block)
    }

    fn visit_block(&mut self, block: &Block) -> Result<(), FormattingError> {
        let Block {
            id,
            open,
            statements,
            close,
        } = block;

        writeln!(self.writer, "{{")?;
        self.writer.indent();
        for statement in statements {
            self.visit_statement(statement)?;
            write!(self.writer, "\n")?;
        }
        self.writer.dedent();
        writeln!(self.writer, "}}")?;

        Ok(())
    }

    fn visit_statement(&mut self, statement: &Stmt) -> Result<(), FormattingError> {
        match statement {
            Stmt::Local(local) => self.visit_local(local),
            Stmt::Item(item, semi) => self.visit_item(item, *semi),
            Stmt::Expr(expr) => self.visit_expr(expr),
            Stmt::Semi(semi) => {
                let StmtSemi {
                    expr,
                    semi_token,
                    needs_semi,
                } = semi;
                self.visit_expr(expr)?;
                match needs_semi {
                    Some(true) => write!(self.writer, ";")?,
                    Some(false) => {}
                    None => write!(self.writer, ";")?,
                }

                Ok(())
            }
        }
    }

    fn visit_local(&mut self, local: &Local) -> Result<(), FormattingError> {
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

        write!(self.writer, "let ")?;
        self.visit_pattern(pat)?;
        write!(self.writer, " = ")?;
        self.visit_expr(expr)?;
        write!(self.writer, ";")?;

        Ok(())
    }

    fn visit_unary(&mut self, unary: &ExprUnary) -> Result<(), FormattingError> {
        let ExprUnary {
            op,
            expr,
            attributes,
        } = unary;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        write!(self.writer, "{}", op)?;

        self.visit_expr(expr)
    }

    fn visit_group(&mut self, group: &ExprGroup) -> Result<(), FormattingError> {
        let ExprGroup {
            expr,
            attributes,
            open,
            close,
        } = group;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        write!(self.writer, "(")?;
        self.visit_expr(expr)?;
        write!(self.writer, ")")?;

        Ok(())
    }

    fn visit_path(&mut self, path: &Path) -> Result<(), FormattingError> {
        let Path {
            id,
            global,
            first,
            rest,
            trailing,
        } = path;

        if global.is_some() {
            write!(self.writer, "::")?;
        }

        self.visit_path_segment(first)?;
        for (_, segment) in rest {
            write!(self.writer, "::")?;
            self.visit_path_segment(segment)?;
        }

        if let Some(trailing) = trailing {
            write!(self.writer, "::")?;
        }

        Ok(())
    }

    fn visit_lit(&mut self, lit: &ExprLit) -> Result<(), FormattingError> {
        let ExprLit { attributes, lit } = lit;

        for attribute in attributes {
            self.visit_attribute(attribute)?;
        }

        match lit {
            crate::ast::Lit::Bool(b) => {
                let s = self.resolve(b.span)?;
                write!(self.writer, "{}", s)?
            }
            crate::ast::Lit::Byte(val) => {
                let s = self.resolve(val.span)?;
                write!(self.writer, "{}", s)?
            }
            crate::ast::Lit::Str(v) => {
                let s = self.resolve(v.span)?;
                write!(self.writer, "{}", s)?
            }
            crate::ast::Lit::ByteStr(v) => {
                let s = self.resolve(v.span)?;
                write!(self.writer, "{}", s)?
            }
            crate::ast::Lit::Char(c) => {
                let s = self.resolve(c.span)?;
                write!(self.writer, "{}", s)?
            }
            crate::ast::Lit::Number(n) => {
                let s = self.resolve(n.span)?;
                write!(self.writer, "{}", s)?
            }
        }
        Ok(())
    }

    fn visit_binary(&mut self, binary: &ExprBinary) -> Result<(), FormattingError> {
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
        write!(self.writer, " {} ", op)?;
        self.visit_expr(rhs.as_ref())?;
        Ok(())
    }

    fn visit_path_segment_expr(&mut self, expr: &PathSegmentExpr) -> Result<(), FormattingError> {
        let PathSegmentExpr { expr } = expr;
        self.visit_expr(expr)
    }

    fn visit_item_use_segment(
        &mut self,
        segment: &crate::ast::ItemUseSegment,
    ) -> Result<(), FormattingError> {
        match segment {
            crate::ast::ItemUseSegment::PathSegment(path) => {
                self.visit_path_segment(path)?;
            }
            crate::ast::ItemUseSegment::Wildcard(star) => write!(self.writer, "*")?,
            crate::ast::ItemUseSegment::Group(braced_group) => {
                let Braced {
                    open,
                    braced,
                    close,
                } = braced_group;
                write!(self.writer, "{{")?;
                for (item, comma) in braced {
                    self.visit_item_use_path(item, *comma)?;
                    write!(self.writer, ", ")?;
                }

                write!(self.writer, "}}")?;
            }
        }

        Ok(())
    }

    fn emit_visibility(
        &mut self,
        visibility: &crate::ast::Visibility,
    ) -> Result<(), FormattingError> {
        match visibility {
            crate::ast::Visibility::Public(token) => write!(self.writer, "pub ")?,
            crate::ast::Visibility::Inherited => {}
            crate::ast::Visibility::Crate(_) => write!(self.writer, "pub(crate) ")?,
            crate::ast::Visibility::Super(_) => write!(self.writer, "pub(super) ")?,
            crate::ast::Visibility::SelfValue(_) => write!(self.writer, "pub(self) ")?,
            crate::ast::Visibility::In(target) => {
                write!(self.writer, "pub(in ")?;
                self.visit_path(&target.restriction.path)?;
                write!(self.writer, ") ")?;
            }
        }

        Ok(())
    }
}

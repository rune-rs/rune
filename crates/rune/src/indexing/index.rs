use crate::alloc::prelude::*;
use crate::alloc::{Box, Vec, VecDeque};
use crate::ast::{self, OptionSpanned, Spanned};
use crate::compile::attrs;
use crate::compile::meta;
use crate::compile::{self, Doc, DynLocation, ErrorKind, Location, Visibility, WithSpan};
use crate::indexing::{self, Indexed, Layer};
use crate::parse::Resolve;
use crate::query::{ItemImplEntry, QueryImplFn};
use crate::runtime::Call;
use crate::worker::{Import, ImportKind};

use super::{ast_to_visibility, Indexer};

use rune_macros::instrument;

/// Macros are only allowed to expand recursively into other macros 64 times.
const MAX_MACRO_RECURSION: usize = 64;

/// Index the contents of a module known by its AST as a "file".
pub(crate) fn file(idx: &mut Indexer<'_, '_>, ast: &mut ast::File) -> compile::Result<()> {
    let mut p = attrs::Parser::new(&ast.attributes)?;

    // This part catches comments interior to the module of the form `//!`.
    for doc in p.parse_all::<attrs::Doc>(resolve_context!(idx.q), &ast.attributes)? {
        let (span, doc) = doc?;

        let doc_string = doc.doc_string.resolve(resolve_context!(idx.q))?;

        idx.q
            .visitor
            .visit_doc_comment(
                &DynLocation::new(idx.source_id, &span),
                idx.q.pool.module_item(idx.item.module),
                idx.q.pool.module_item_hash(idx.item.module),
                &doc_string,
            )
            .with_span(span)?;
    }

    if let Some(first) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            first,
            "File attributes are not supported",
        ));
    }

    // Items take priority.
    let mut head = VecDeque::new();

    // Macros and items with attributes are expanded as they are encountered, but after regular items have
    // been processed.
    let mut queue = VecDeque::new();

    for (item, semi) in ast.items.drain(..) {
        match item {
            i @ ast::Item::MacroCall(_) => {
                queue.try_push_back((0, i, Vec::new(), semi))?;
            }
            i if !i.attributes().is_empty() => {
                queue.try_push_back((0, i, Vec::new(), semi))?;
            }
            i => {
                head.try_push_back((i, semi))?;
            }
        }
    }

    'uses: while !head.is_empty() || !queue.is_empty() {
        while let Some((i, semi)) = head.pop_front() {
            if let Some(semi) = semi {
                if !i.needs_semi_colon() {
                    idx.q
                        .diagnostics
                        .unnecessary_semi_colon(idx.source_id, &semi)?;
                }
            }

            item(idx, i)?;
        }

        while let Some((depth, mut item, mut skipped_attributes, semi)) = queue.pop_front() {
            if depth >= MAX_MACRO_RECURSION {
                return Err(compile::Error::new(
                    &item,
                    ErrorKind::MaxMacroRecursion {
                        depth,
                        max: MAX_MACRO_RECURSION,
                    },
                ));
            }

            // Before further processing all attributes are either expanded, or
            // if unknown put in `skipped_attributes`, to either be reinserted
            // for the `item` handler or to be used by the macro_call expansion
            // below.
            if let Some(mut attr) = item.remove_first_attribute() {
                let Some(file) = idx.expand_attribute_macro::<ast::File>(&mut attr, &item)? else {
                    skipped_attributes.try_push(attr)?;

                    if !matches!(item, ast::Item::MacroCall(_)) && item.attributes().is_empty() {
                        // For all we know only non macro attributes remain, which will be
                        // handled by the item handler.
                        *item.attributes_mut() = skipped_attributes;
                        head.try_push_front((item, semi))?;
                    } else {
                        // items with remaining attributes and macro calls will be dealt with by
                        // reinserting in the queue.
                        queue.try_push_back((depth, item, skipped_attributes, semi))?;
                    }

                    continue;
                };

                for (item, semi) in file.items.into_iter().rev() {
                    match item {
                        item @ ast::Item::MacroCall(_) => {
                            queue.try_push_back((depth.wrapping_add(1), item, Vec::new(), semi))?;
                        }
                        item if !item.attributes().is_empty() => {
                            queue.try_push_back((depth.wrapping_add(1), item, Vec::new(), semi))?;
                        }
                        item => {
                            head.try_push_front((item, semi))?;
                        }
                    }
                }

                continue;
            }

            let ast::Item::MacroCall(mut macro_call) = item else {
                return Err(compile::Error::msg(
                    &item,
                    "Expected attributes on macro call",
                ));
            };

            macro_call.attributes = skipped_attributes;

            let mut p = attrs::Parser::new(&macro_call.attributes)?;

            if idx.try_expand_internal_macro(&mut p, &mut macro_call)? {
                if let Some(attr) = p.remaining(&macro_call.attributes).next() {
                    return Err(compile::Error::msg(
                        attr,
                        "Attributes on macros are not supported",
                    ));
                }

                // Macro call must be added to output to make sure its instructions are assembled.
                ast.items
                    .try_push((ast::Item::MacroCall(macro_call), semi))?;
            } else {
                if let Some(attr) = p.remaining(&macro_call.attributes).next() {
                    return Err(compile::Error::msg(
                        attr,
                        "Attributes on macros are not supported",
                    ));
                }

                let file = idx.expand_macro::<ast::File>(&mut macro_call)?;

                for (item, semi) in file.items.into_iter().rev() {
                    match item {
                        item @ ast::Item::MacroCall(_) => {
                            queue.try_push_back((depth.wrapping_add(1), item, Vec::new(), semi))?;
                        }
                        item if !item.attributes().is_empty() => {
                            queue.try_push_back((depth.wrapping_add(1), item, Vec::new(), semi))?;
                        }
                        item => {
                            head.try_push_front((item, semi))?;
                        }
                    }
                }
            }

            if !head.is_empty() {
                continue 'uses;
            }
        }
    }

    Ok(())
}

#[instrument(span = span)]
pub(crate) fn empty_block_fn(
    idx: &mut Indexer<'_, '_>,
    mut ast: ast::EmptyBlock,
    span: &dyn Spanned,
) -> compile::Result<()> {
    let idx_item = idx.item.replace();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, span),
        idx.item.module,
        Visibility::Public,
        &[],
    )?;

    idx.scopes.push()?;

    statements(idx, &mut ast.statements)?;

    idx.item = idx_item;

    let layer = idx.scopes.pop().with_span(span)?;

    let call = match (layer.awaits.is_empty(), layer.yields.is_empty()) {
        (true, true) => Call::Immediate,
        (false, true) => Call::Async,
        (true, false) => Call::Generator,
        (false, false) => Call::Stream,
    };

    idx.q.index_and_build(indexing::Entry {
        item_meta,
        indexed: Indexed::Function(indexing::Function {
            ast: indexing::FunctionAst::Empty(Box::try_new(ast)?, span.span()),
            call,
            is_instance: false,
            is_test: false,
            is_bench: false,
            impl_item: None,
        }),
    })?;

    Ok(())
}

#[instrument(span = ast)]
pub(crate) fn item_fn_immediate(
    idx: &mut Indexer<'_, '_>,
    mut ast: ast::ItemFn,
) -> compile::Result<()> {
    let name = ast.name.resolve(resolve_context!(idx.q))?;

    let visibility = ast_to_visibility(&ast.visibility)?;

    let mut p = attrs::Parser::new(&ast.attributes)?;

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &ast.attributes)?;

    let guard = idx.items.push_name(name.as_ref())?;
    let idx_item = idx.item.replace();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        visibility,
        &docs,
    )?;

    idx.scopes.push()?;

    for (arg, _) in &mut ast.args {
        if let ast::FnArg::Pat(p) = arg {
            pat(idx, p)?;
        }
    }

    // Take and restore item nesting.
    let last = idx.nested_item.replace(ast.descriptive_span());
    block(idx, &mut ast.body)?;
    idx.nested_item = last;

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;

    let layer = idx.scopes.pop().with_span(&ast)?;

    if let (Some(const_token), Some(async_token)) = (ast.const_token, ast.async_token) {
        return Err(compile::Error::new(
            const_token.span().join(async_token.span()),
            ErrorKind::FnConstAsyncConflict,
        ));
    };

    let call = validate_call(ast.const_token.as_ref(), ast.async_token.as_ref(), &layer)?;

    let Some(call) = call else {
        idx.q.index_const_fn(item_meta, Box::try_new(ast)?)?;
        return Ok(());
    };

    let is_test = match p.try_parse::<attrs::Test>(resolve_context!(idx.q), &ast.attributes)? {
        Some((attr, _)) => {
            if let Some(_nested_span) = idx.nested_item {
                return Err(compile::Error::new(
                    attr,
                    ErrorKind::NestedTest {
                        #[cfg(feature = "emit")]
                        nested_span: _nested_span,
                    },
                ));
            }

            true
        }
        _ => false,
    };

    let is_bench = match p.try_parse::<attrs::Bench>(resolve_context!(idx.q), &ast.attributes)? {
        Some((attr, _)) => {
            if let Some(_nested_span) = idx.nested_item {
                let span = attr.span().join(ast.descriptive_span());

                return Err(compile::Error::new(
                    span,
                    ErrorKind::NestedBench {
                        #[cfg(feature = "emit")]
                        nested_span: _nested_span,
                    },
                ));
            }

            true
        }
        _ => false,
    };

    if let Some(attrs) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            attrs,
            "Attributes on functions are not supported",
        ));
    }

    let is_instance = ast.is_instance();

    if is_instance {
        if is_test {
            return Err(compile::Error::msg(
                &ast,
                "The #[test] attribute is not supported on functions receiving `self`",
            ));
        }

        if is_bench {
            return Err(compile::Error::msg(
                &ast,
                "The #[bench] attribute is not supported on functions receiving `self`",
            ));
        }

        if idx.item.impl_item.is_none() {
            return Err(compile::Error::new(
                &ast,
                ErrorKind::InstanceFunctionOutsideImpl,
            ));
        };
    }

    let entry = indexing::Entry {
        item_meta,
        indexed: Indexed::Function(indexing::Function {
            ast: indexing::FunctionAst::Item(Box::try_new(ast)?),
            call,
            is_instance,
            is_test,
            is_bench,
            impl_item: idx.item.impl_item,
        }),
    };

    // It's only a public item in the sense of exporting it if it's not inside
    // of a nested item. Instance functions are always eagerly exported since
    // they need to be accessed dynamically through `self`.
    let is_exported = is_instance
        || item_meta.is_public(idx.q.pool) && idx.nested_item.is_none()
        || is_test
        || is_bench;

    if is_exported {
        idx.q.index_and_build(entry)?;
    } else {
        idx.q.index(entry)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_fn(idx: &mut Indexer<'_, '_>, ast: ast::ItemFn) -> compile::Result<()> {
    if let Some(impl_item) = idx.item.impl_item {
        idx.q
            .inner
            .impl_functions
            .entry(impl_item)
            .or_try_default()?
            .try_push(QueryImplFn {
                ast: Box::try_new(ast)?,
            })?;
    } else {
        item_fn_immediate(idx, ast)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_block(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprBlock) -> compile::Result<()> {
    if let Some(span) = ast.attributes.option_span() {
        return Err(compile::Error::msg(
            span,
            "Attributes on blocks are not supported",
        ));
    }

    if ast.async_token.is_none() && ast.const_token.is_none() {
        if let Some(span) = ast.move_token.option_span() {
            return Err(compile::Error::msg(
                span,
                "The `move` modifier on blocks is not supported",
            ));
        }

        return block(idx, &mut ast.block);
    }

    let guard = idx.items.push_id()?;
    let idx_item = idx.item.replace();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        Visibility::default(),
        &[],
    )?;

    ast.block.id.set(item_meta.id);

    if ast.const_token.is_some() {
        if let Some(async_token) = ast.async_token {
            return Err(compile::Error::new(
                async_token,
                ErrorKind::BlockConstAsyncConflict,
            ));
        }

        block(idx, &mut ast.block)?;
        idx.q.index_const_block(item_meta, &ast.block)?;
    } else {
        idx.scopes.push()?;
        block(idx, &mut ast.block)?;
        let layer = idx.scopes.pop().with_span(&ast)?;

        let call = validate_call(ast.const_token.as_ref(), ast.async_token.as_ref(), &layer)?;

        let Some(call) = call else {
            return Err(compile::Error::new(ast, ErrorKind::ClosureKind));
        };

        idx.q.index_meta(
            &*ast,
            item_meta,
            meta::Kind::AsyncBlock {
                call,
                do_move: ast.move_token.is_some(),
            },
        )?;
    }

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    Ok(())
}

fn statements(idx: &mut Indexer<'_, '_>, ast: &mut Vec<ast::Stmt>) -> compile::Result<()> {
    let mut statements = Vec::new();

    for stmt in ast.drain(..) {
        match stmt {
            ast::Stmt::Item(i, semi) => {
                if let Some(semi) = semi {
                    if !i.needs_semi_colon() {
                        idx.q
                            .diagnostics
                            .unnecessary_semi_colon(idx.source_id, &semi)?;
                    }
                }

                item(idx, i)?;
            }
            stmt => {
                statements.try_push(stmt)?;
            }
        }
    }

    let mut must_be_last = None;

    for stmt in &mut statements {
        if let Some(span) = must_be_last {
            return Err(compile::Error::new(
                span,
                ErrorKind::ExpectedBlockSemiColon {
                    #[cfg(feature = "emit")]
                    followed_span: stmt.span(),
                },
            ));
        }

        match stmt {
            ast::Stmt::Local(l) => {
                local(idx, l)?;
            }
            ast::Stmt::Expr(e) => {
                if e.needs_semi() {
                    must_be_last = Some(e.span());
                }

                expr(idx, e)?;
            }
            ast::Stmt::Semi(semi) => {
                if !semi.needs_semi() {
                    idx.q
                        .diagnostics
                        .unnecessary_semi_colon(idx.source_id, semi)?;
                }

                expr(idx, &mut semi.expr)?;
            }
            ast::Stmt::Item(i, ..) => {
                return Err(compile::Error::msg(i, "Unexpected item in this stage"));
            }
        }
    }

    *ast = statements;
    Ok(())
}

#[instrument(span = ast)]
fn block(idx: &mut Indexer<'_, '_>, ast: &mut ast::Block) -> compile::Result<()> {
    let guard = idx.items.push_id()?;
    let idx_item = idx.item.replace();

    idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        Visibility::Inherited,
        &[],
    )?;

    statements(idx, &mut ast.statements)?;
    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    Ok(())
}

#[instrument(span = ast)]
fn local(idx: &mut Indexer<'_, '_>, ast: &mut ast::Local) -> compile::Result<()> {
    if let Some(span) = ast.attributes.option_span() {
        return Err(compile::Error::msg(
            span,
            "Attributes on local declarations are not supported",
        ));
    }

    if let Some(mut_token) = ast.mut_token {
        return Err(compile::Error::new(mut_token, ErrorKind::UnsupportedMut));
    }

    // We index the rhs expression first so that it doesn't see it's own
    // declaration and use that instead of capturing from the outside.
    expr(idx, &mut ast.expr)?;
    pat(idx, &mut ast.pat)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_let(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprLet) -> compile::Result<()> {
    if let Some(mut_token) = ast.mut_token {
        return Err(compile::Error::new(mut_token, ErrorKind::UnsupportedMut));
    }

    pat(idx, &mut ast.pat)?;
    expr(idx, &mut ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn pat(idx: &mut Indexer<'_, '_>, ast: &mut ast::Pat) -> compile::Result<()> {
    match ast {
        ast::Pat::Path(pat) => {
            path(idx, &mut pat.path)?;
        }
        ast::Pat::Object(pat) => {
            pat_object(idx, pat)?;
        }
        ast::Pat::Vec(pat) => {
            pat_vec(idx, pat)?;
        }
        ast::Pat::Tuple(pat) => {
            pat_tuple(idx, pat)?;
        }
        ast::Pat::Binding(pat) => {
            pat_binding(idx, pat)?;
        }
        ast::Pat::Ignore(..) => (),
        ast::Pat::Lit(..) => (),
        ast::Pat::Rest(..) => (),
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_tuple(idx: &mut Indexer<'_, '_>, ast: &mut ast::PatTuple) -> compile::Result<()> {
    if let Some(p) = &mut ast.path {
        path(idx, p)?;
    }

    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_object(idx: &mut Indexer<'_, '_>, ast: &mut ast::PatObject) -> compile::Result<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Anonymous(..) => (),
        ast::ObjectIdent::Named(p) => {
            path(idx, p)?;
        }
    }

    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_vec(idx: &mut Indexer<'_, '_>, ast: &mut ast::PatVec) -> compile::Result<()> {
    for (p, _) in &mut ast.items {
        pat(idx, p)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_path(idx: &mut Indexer<'_, '_>, ast: &mut ast::PatPath) -> compile::Result<()> {
    path(idx, &mut ast.path)?;
    Ok(())
}

#[instrument(span = ast)]
fn pat_binding(idx: &mut Indexer<'_, '_>, ast: &mut ast::PatBinding) -> compile::Result<()> {
    pat(idx, &mut ast.pat)?;
    Ok(())
}

#[instrument(span = ast)]
pub(crate) fn expr(idx: &mut Indexer<'_, '_>, ast: &mut ast::Expr) -> compile::Result<()> {
    match ast {
        ast::Expr::Path(e) => {
            path(idx, e)?;
        }
        ast::Expr::Let(e) => {
            expr_let(idx, e)?;
        }
        ast::Expr::Block(e) => {
            expr_block(idx, e)?;
        }
        ast::Expr::Group(e) => {
            expr(idx, &mut e.expr)?;
        }
        ast::Expr::Empty(e) => {
            expr(idx, &mut e.expr)?;
        }
        ast::Expr::If(e) => {
            expr_if(idx, e)?;
        }
        ast::Expr::Assign(e) => {
            expr_assign(idx, e)?;
        }
        ast::Expr::Binary(e) => {
            expr_binary(idx, e)?;
        }
        ast::Expr::Match(e) => {
            expr_match(idx, e)?;
        }
        ast::Expr::Closure(e) => {
            expr_closure(idx, e)?;
        }
        ast::Expr::While(e) => {
            expr_while(idx, e)?;
        }
        ast::Expr::Loop(e) => {
            expr_loop(idx, e)?;
        }
        ast::Expr::For(e) => {
            expr_for(idx, e)?;
        }
        ast::Expr::FieldAccess(e) => {
            expr_field_access(idx, e)?;
        }
        ast::Expr::Unary(e) => {
            expr_unary(idx, e)?;
        }
        ast::Expr::Index(e) => {
            expr_index(idx, e)?;
        }
        ast::Expr::Break(e) => {
            expr_break(idx, e)?;
        }
        ast::Expr::Continue(e) => {
            expr_continue(idx, e)?;
        }
        ast::Expr::Yield(e) => {
            expr_yield(idx, e)?;
        }
        ast::Expr::Return(e) => {
            expr_return(idx, e)?;
        }
        ast::Expr::Await(e) => {
            expr_await(idx, e)?;
        }
        ast::Expr::Try(e) => {
            expr_try(idx, e)?;
        }
        ast::Expr::Select(e) => {
            expr_select(idx, e)?;
        }
        // ignored because they have no effect on indexing.
        ast::Expr::Call(e) => {
            expr_call(idx, e)?;
        }
        ast::Expr::Lit(..) => {}
        ast::Expr::Tuple(e) => {
            expr_tuple(idx, e)?;
        }
        ast::Expr::Vec(e) => {
            expr_vec(idx, e)?;
        }
        ast::Expr::Object(e) => {
            expr_object(idx, e)?;
        }
        ast::Expr::Range(e) => {
            expr_range(idx, e)?;
        }
        // NB: macros have nothing to index, they don't export language
        // items.
        ast::Expr::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with statements for
            // which the macro **might** have been expanded to a built-in macro
            // if we end up here. So instead of expanding if the id is set, we
            // just assert that the builtin macro has been added to the query
            // engine.

            if !macro_call.id.is_set() {
                let mut p = attrs::Parser::new(&macro_call.attributes)?;

                let expanded = idx.try_expand_internal_macro(&mut p, macro_call)?;

                if let Some(span) = p.remaining(&macro_call.attributes).next() {
                    return Err(compile::Error::msg(span, "Unsupported macro attribute"));
                }

                if !expanded {
                    let out = idx.expand_macro::<ast::Expr>(macro_call)?;
                    idx.enter_macro(&macro_call)?;
                    *ast = out;
                    expr(idx, ast)?;
                    idx.leave_macro();
                }
            } else {
                // Assert that the built-in macro has been expanded.
                idx.q
                    .builtin_macro_for(&*macro_call)
                    .with_span(&*macro_call)?;
            }

            return Ok(());
        }
    }

    if let [first, ..] = ast.attributes() {
        return Err(compile::Error::msg(
            first,
            "Attributes on expressions are not supported",
        ));
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_if(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprIf) -> compile::Result<()> {
    condition(idx, &mut ast.condition)?;
    block(idx, &mut ast.block)?;

    for expr_else_if in &mut ast.expr_else_ifs {
        condition(idx, &mut expr_else_if.condition)?;
        block(idx, &mut expr_else_if.block)?;
    }

    if let Some(expr_else) = &mut ast.expr_else {
        block(idx, &mut expr_else.block)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_assign(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprAssign) -> compile::Result<()> {
    expr(idx, &mut ast.lhs)?;
    expr(idx, &mut ast.rhs)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_binary(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprBinary) -> compile::Result<()> {
    expr(idx, &mut ast.lhs)?;
    expr(idx, &mut ast.rhs)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_match(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprMatch) -> compile::Result<()> {
    expr(idx, &mut ast.expr)?;

    for (branch, _) in &mut ast.branches {
        if let Some((_, condition)) = &mut branch.condition {
            expr(idx, condition)?;
        }

        pat(idx, &mut branch.pat)?;
        expr(idx, &mut branch.body)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn condition(idx: &mut Indexer<'_, '_>, ast: &mut ast::Condition) -> compile::Result<()> {
    match ast {
        ast::Condition::Expr(e) => {
            expr(idx, e)?;
        }
        ast::Condition::ExprLet(e) => {
            expr_let(idx, e)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_enum(idx: &mut Indexer<'_, '_>, mut ast: ast::ItemEnum) -> compile::Result<()> {
    let mut p = attrs::Parser::new(&ast.attributes)?;

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &ast.attributes)?;

    if let Some(first) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            first,
            "Attributes on enums are not supported",
        ));
    }

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let guard = idx.items.push_name(name.as_ref())?;
    let idx_item = idx.item.replace();

    let visibility = ast_to_visibility(&ast.visibility)?;
    let enum_item = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        visibility,
        &docs,
    )?;

    idx.q.index_enum(enum_item)?;

    for (index, (mut variant, _)) in ast.variants.drain().enumerate() {
        let mut p = attrs::Parser::new(&variant.attributes)?;

        let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &variant.attributes)?;

        if let Some(first) = p.remaining(&variant.attributes).next() {
            return Err(compile::Error::msg(
                first,
                "Attributes on variants are not supported",
            ));
        }

        let name = variant.name.resolve(resolve_context!(idx.q))?;
        let guard = idx.items.push_name(name.as_ref())?;
        let idx_item = idx.item.replace();

        let item_meta = idx.q.insert_new_item(
            &idx.items,
            &DynLocation::new(idx.source_id, &variant.name),
            idx.item.module,
            Visibility::Public,
            &docs,
        )?;

        variant.id.set(item_meta.id);

        let cx = resolve_context!(idx.q);

        for (field, _) in variant.body.fields() {
            let mut p = attrs::Parser::new(&field.attributes)?;
            let docs = Doc::collect_from(cx, &mut p, &field.attributes)?;

            if let Some(first) = p.remaining(&field.attributes).next() {
                return Err(compile::Error::msg(
                    first,
                    "Attributes on variant fields are not supported",
                ));
            }

            let name = field.name.resolve(cx)?;

            for doc in docs {
                idx.q
                    .visitor
                    .visit_field_doc_comment(
                        &DynLocation::new(idx.source_id, &doc),
                        idx.q.pool.item(item_meta.item),
                        idx.q.pool.item_type_hash(item_meta.item),
                        name,
                        doc.doc_string.resolve(cx)?.as_ref(),
                    )
                    .with_span(doc)?;
            }
        }

        idx.item = idx_item;
        idx.items.pop(guard).with_span(&variant)?;
        idx.q
            .index_variant(item_meta, enum_item.id, variant, index)?;
    }

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    Ok(())
}

#[instrument(span = ast)]
fn item_struct(idx: &mut Indexer<'_, '_>, mut ast: ast::ItemStruct) -> compile::Result<()> {
    let mut p = attrs::Parser::new(&ast.attributes)?;

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &ast.attributes)?;

    if let Some(first) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            first,
            "Attributes on structs are not supported",
        ));
    }

    let ident = ast.ident.resolve(resolve_context!(idx.q))?;
    let guard = idx.items.push_name(ident)?;
    let idx_item = idx.item.replace();

    let visibility = ast_to_visibility(&ast.visibility)?;
    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        visibility,
        &docs,
    )?;
    ast.id.set(item_meta.id);

    let cx = resolve_context!(idx.q);

    for (field, _) in ast.body.fields() {
        let mut p = attrs::Parser::new(&field.attributes)?;
        let docs = Doc::collect_from(cx, &mut p, &field.attributes)?;

        if let Some(first) = p.remaining(&field.attributes).next() {
            return Err(compile::Error::msg(
                first,
                "Attributes on fields are not supported",
            ));
        }

        let name = field.name.resolve(cx)?;

        for doc in docs {
            idx.q
                .visitor
                .visit_field_doc_comment(
                    &DynLocation::new(idx.source_id, &doc),
                    idx.q.pool.item(item_meta.item),
                    idx.q.pool.item_type_hash(item_meta.item),
                    name,
                    doc.doc_string.resolve(cx)?.as_ref(),
                )
                .with_span(doc)?;
        }

        if !field.visibility.is_inherited() {
            return Err(compile::Error::msg(
                field,
                "Field visibility is not supported",
            ));
        }
    }

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    idx.q.index_struct(item_meta, Box::try_new(ast)?)?;
    Ok(())
}

#[instrument(span = ast)]
fn item_impl(idx: &mut Indexer<'_, '_>, mut ast: ast::ItemImpl) -> compile::Result<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(compile::Error::msg(
            first,
            "Attributes on impl blocks are not supported",
        ));
    }

    path(idx, &mut ast.path)?;

    let location = Location::new(idx.source_id, ast.path.span());
    let id = idx.q.gen.next();

    idx.q.inner.impl_item_queue.try_push_back(ItemImplEntry {
        path: Box::try_new(ast.path)?,
        location,
        id,
        root: idx.root.clone(),
        nested_item: idx.nested_item,
        macro_depth: idx.macro_depth,
    })?;

    let idx_item = idx.item.replace_impl(id);

    for i in ast.functions.drain(..) {
        item_fn(idx, i)?;
    }

    idx.item = idx_item;
    Ok(())
}

#[instrument(span = ast)]
fn item_mod(idx: &mut Indexer<'_, '_>, mut ast: ast::ItemMod) -> compile::Result<()> {
    let mut p = attrs::Parser::new(&ast.attributes)?;

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &ast.attributes)?;

    if let Some(first) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            first,
            "Attributes on modules are not supported",
        ));
    }

    let name_span = ast.name_span();

    match &mut ast.body {
        ast::ItemModBody::EmptyBody(..) => {
            idx.handle_file_mod(&mut ast, &docs)?;
        }
        ast::ItemModBody::InlineBody(body) => {
            let name = ast.name.resolve(resolve_context!(idx.q))?;
            let guard = idx.items.push_name(name.as_ref())?;
            let idx_item = idx.item.replace();

            let visibility = ast_to_visibility(&ast.visibility)?;
            let mod_item = idx.q.insert_mod(
                &idx.items,
                &DynLocation::new(idx.source_id, name_span),
                idx.item.module,
                visibility,
                &docs,
            )?;

            ast.id.set(idx.items.id().with_span(name_span)?);

            {
                let idx_item = idx.item.replace_module(mod_item);
                file(idx, &mut body.file)?;
                idx.item.module = mod_item;
                idx.item = idx_item;
            }

            idx.item = idx_item;
            idx.items.pop(guard).with_span(&ast)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_const(idx: &mut Indexer<'_, '_>, mut ast: ast::ItemConst) -> compile::Result<()> {
    let mut p = attrs::Parser::new(&ast.attributes)?;

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut p, &ast.attributes)?;

    if let Some(first) = p.remaining(&ast.attributes).next() {
        return Err(compile::Error::msg(
            first,
            "Attributes on constants are not supported",
        ));
    }

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let guard = idx.items.push_name(name.as_ref())?;
    let idx_item = idx.item.replace();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &ast),
        idx.item.module,
        ast_to_visibility(&ast.visibility)?,
        &docs,
    )?;

    ast.id.set(item_meta.id);

    let last = idx.nested_item.replace(ast.descriptive_span());
    expr(idx, &mut ast.expr)?;
    idx.nested_item = last;
    idx.q.index_const_expr(item_meta, &ast.expr)?;

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    Ok(())
}

#[instrument(span = ast)]
fn item(idx: &mut Indexer<'_, '_>, ast: ast::Item) -> compile::Result<()> {
    match ast {
        ast::Item::Enum(item) => {
            item_enum(idx, item)?;
        }
        ast::Item::Struct(item) => {
            item_struct(idx, item)?;
        }
        ast::Item::Fn(item) => {
            item_fn(idx, item)?;
        }
        ast::Item::Impl(item) => {
            item_impl(idx, item)?;
        }
        ast::Item::Mod(item) => {
            item_mod(idx, item)?;
        }
        ast::Item::Const(item) => {
            item_const(idx, item)?;
        }
        ast::Item::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with items for
            // which the macro must have been expanded to a built-in macro
            // if we end up here. So instead of expanding here, we just
            // assert that the builtin macro has been added to the query
            // engine.

            // Assert that the built-in macro has been expanded.
            idx.q
                .builtin_macro_for(&macro_call)
                .with_span(&macro_call)?;

            if let Some(span) = macro_call.attributes.first() {
                return Err(compile::Error::msg(
                    span,
                    "Attributes on macros are not supported",
                ));
            }
        }
        // NB: imports are ignored during indexing.
        ast::Item::Use(item_use) => {
            if let Some(span) = item_use.attributes.first() {
                return Err(compile::Error::msg(
                    span,
                    "Attributes on uses are not supported",
                ));
            }

            let Some(queue) = idx.queue.as_mut() else {
                return Err(compile::Error::msg(
                    &item_use,
                    "Imports are not supported in this context",
                ));
            };

            let visibility = ast_to_visibility(&item_use.visibility)?;

            let import = Import {
                kind: ImportKind::Global,
                visibility,
                module: idx.item.module,
                item: idx.items.item().try_clone()?,
                source_id: idx.source_id,
                ast: Box::try_new(item_use)?,
            };

            import.process(&mut idx.q, &mut |task| {
                queue.try_push_back(task)?;
                Ok(())
            })?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn path(idx: &mut Indexer<'_, '_>, ast: &mut ast::Path) -> compile::Result<()> {
    ast.id.set(idx.item_id()?);

    path_segment(idx, &mut ast.first)?;

    for (_, segment) in &mut ast.rest {
        path_segment(idx, segment)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn path_segment(idx: &mut Indexer<'_, '_>, ast: &mut ast::PathSegment) -> compile::Result<()> {
    if let ast::PathSegment::Generics(generics) = ast {
        for (param, _) in generics {
            expr(idx, &mut param.expr)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_while(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprWhile) -> compile::Result<()> {
    condition(idx, &mut ast.condition)?;
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_loop(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprLoop) -> compile::Result<()> {
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_for(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprFor) -> compile::Result<()> {
    expr(idx, &mut ast.iter)?;
    pat(idx, &mut ast.binding)?;
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_closure(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprClosure) -> compile::Result<()> {
    let guard = idx.items.push_id()?;
    let idx_item = idx.item.replace();

    idx.scopes.push()?;

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        &DynLocation::new(idx.source_id, &*ast),
        idx.item.module,
        Visibility::Inherited,
        &[],
    )?;

    ast.id.set(item_meta.id);

    for (arg, _) in ast.args.as_slice_mut() {
        match arg {
            ast::FnArg::SelfValue(s) => {
                return Err(compile::Error::new(s, ErrorKind::UnsupportedSelf));
            }
            ast::FnArg::Pat(p) => {
                pat(idx, p)?;
            }
        }
    }

    expr(idx, &mut ast.body)?;

    let layer = idx.scopes.pop().with_span(&*ast)?;

    let call = validate_call(None, ast.async_token.as_ref(), &layer)?;

    let Some(call) = call else {
        return Err(compile::Error::new(&*ast, ErrorKind::ClosureKind));
    };

    idx.q.index_meta(
        ast,
        item_meta,
        meta::Kind::Closure {
            call,
            do_move: ast.move_token.is_some(),
        },
    )?;

    idx.item = idx_item;
    idx.items.pop(guard).with_span(&ast)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_field_access(
    idx: &mut Indexer<'_, '_>,
    ast: &mut ast::ExprFieldAccess,
) -> compile::Result<()> {
    expr(idx, &mut ast.expr)?;

    match &mut ast.expr_field {
        ast::ExprField::Path(p) => {
            path(idx, p)?;
        }
        ast::ExprField::LitNumber(..) => {}
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_unary(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprUnary) -> compile::Result<()> {
    expr(idx, &mut ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_index(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprIndex) -> compile::Result<()> {
    expr(idx, &mut ast.index)?;
    expr(idx, &mut ast.target)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_break(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprBreak) -> compile::Result<()> {
    if let Some(e) = ast.expr.as_deref_mut() {
        expr(idx, e)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_continue(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprContinue) -> compile::Result<()> {
    Ok(())
}

#[instrument(span = ast)]
fn expr_yield(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprYield) -> compile::Result<()> {
    let l = idx.scopes.mark().with_span(&*ast)?;
    l.yields.try_push(ast.span())?;

    if let Some(e) = &mut ast.expr {
        expr(idx, e)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_return(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprReturn) -> compile::Result<()> {
    if let Some(e) = &mut ast.expr {
        expr(idx, e)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_await(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprAwait) -> compile::Result<()> {
    let l = idx.scopes.mark().with_span(&*ast)?;
    l.awaits.try_push(ast.span())?;

    expr(idx, &mut ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_try(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprTry) -> compile::Result<()> {
    expr(idx, &mut ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_select(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprSelect) -> compile::Result<()> {
    let l = idx.scopes.mark().with_span(&*ast)?;
    l.awaits.try_push(ast.span())?;

    for (branch, _) in &mut ast.branches {
        match branch {
            ast::ExprSelectBranch::Pat(p) => {
                expr(idx, &mut p.expr)?;
                pat(idx, &mut p.pat)?;
                expr(idx, &mut p.body)?;
            }
            ast::ExprSelectBranch::Default(def) => {
                expr(idx, &mut def.body)?;
            }
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_call(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprCall) -> compile::Result<()> {
    ast.id.set(idx.items.id().with_span(&*ast)?);

    for (e, _) in &mut ast.args {
        expr(idx, e)?;
    }

    expr(idx, &mut ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_tuple(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprTuple) -> compile::Result<()> {
    for (e, _) in &mut ast.items {
        expr(idx, e)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_vec(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprVec) -> compile::Result<()> {
    for (e, _) in &mut ast.items {
        expr(idx, e)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_object(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprObject) -> compile::Result<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Named(p) => {
            // Not a variable use: Name of the object.
            path(idx, p)?;
        }
        ast::ObjectIdent::Anonymous(..) => (),
    }

    for (assign, _) in &mut ast.assignments {
        if let Some((_, e)) = &mut assign.assign {
            expr(idx, e)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_range(idx: &mut Indexer<'_, '_>, ast: &mut ast::ExprRange) -> compile::Result<()> {
    if let Some(from) = &mut ast.start {
        expr(idx, from)?;
    }

    if let Some(to) = &mut ast.end {
        expr(idx, to)?;
    }

    Ok(())
}

/// Construct the calling convention based on the parameters.
fn validate_call(
    const_token: Option<&T![const]>,
    async_token: Option<&T![async]>,
    layer: &Layer,
) -> compile::Result<Option<Call>> {
    for span in &layer.awaits {
        if const_token.is_some() {
            return Err(compile::Error::new(span, ErrorKind::AwaitInConst));
        }

        if async_token.is_none() {
            return Err(compile::Error::new(span, ErrorKind::AwaitOutsideAsync));
        }
    }

    for span in &layer.yields {
        if const_token.is_some() {
            return Err(compile::Error::new(span, ErrorKind::YieldInConst));
        }
    }

    if const_token.is_some() {
        return Ok(None);
    }

    Ok(match (!layer.yields.is_empty(), async_token) {
        (true, None) => Some(Call::Generator),
        (false, None) => Some(Call::Immediate),
        (true, Some(..)) => Some(Call::Stream),
        (false, Some(..)) => Some(Call::Async),
    })
}

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::collections::{HashMap, HashSet};
use crate::compile::v1::{Assembler, Loop, Needs, Scope, Var};
use crate::compile::{
    CaptureMeta, CompileError, CompileErrorKind, CompileResult, Item, PrivMeta, PrivMetaKind,
    PrivStructMeta, PrivVariantMeta,
};
use crate::hash::ParametersBuilder;
use crate::parse::{Id, ParseErrorKind, Resolve};
use crate::query::{BuiltInFormat, BuiltInTemplate, Named};
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget, InstValue,
    InstVariant, Label, PanicReason, Protocol, TypeCheck,
};
use crate::Hash;
use rune_macros::__instrument_ast as instrument;
use std::convert::TryFrom;

/// `self` variable.
const SELF: &str = "self";

#[derive(Debug)]
#[must_use = "must be consumed to make sure the value is realized"]
struct Asm {
    span: Span,
    kind: AsmKind,
}

impl Asm {
    /// Construct an assembly result that leaves the value on the top of the
    /// stack.
    fn top(span: Span) -> Self {
        Self {
            span,
            kind: AsmKind::Top,
        }
    }

    fn var(span: Span, var: Var, local: Box<str>) -> Self {
        Self {
            span,
            kind: AsmKind::Var(var, local),
        }
    }
}

#[derive(Debug)]
pub(crate) enum AsmKind {
    // Result is pushed onto the top of the stack.
    Top,
    // Result belongs to the the given stack offset.
    Var(Var, Box<str>),
}

impl Asm {
    /// Assemble into an instruction.
    fn apply(self, c: &mut Assembler) -> CompileResult<()> {
        match self.kind {
            AsmKind::Top => (),
            AsmKind::Var(var, local) => {
                var.copy(c, self.span, format!("var `{}`", local));
            }
        }

        Ok(())
    }

    /// Assemble into an instruction declaring an anonymous variable if appropriate.
    fn apply_targeted(self, c: &mut Assembler) -> CompileResult<InstAddress> {
        let address = match self.kind {
            AsmKind::Top => {
                c.scopes.decl_anon(self.span)?;
                InstAddress::Top
            }
            AsmKind::Var(var, ..) => InstAddress::Offset(var.offset),
        };

        Ok(address)
    }
}

/// Compile an item.
#[instrument]
fn meta(
    span: Span,
    c: &mut Assembler<'_>,
    meta: &PrivMeta,
    needs: Needs,
    named: Named<'_>,
) -> CompileResult<()> {
    if let Needs::Value = needs {
        match &meta.kind {
            PrivMetaKind::Struct {
                type_hash,
                variant: PrivVariantMeta::Unit,
                ..
            }
            | PrivMetaKind::Variant {
                type_hash,
                variant: PrivVariantMeta::Unit,
                ..
            } => {
                named.assert_not_generic()?;
                c.asm.push_with_comment(
                    Inst::Call {
                        hash: *type_hash,
                        args: 0,
                    },
                    span,
                    meta.info().to_string(),
                );
            }
            PrivMetaKind::Variant {
                variant: PrivVariantMeta::Tuple(tuple),
                ..
            }
            | PrivMetaKind::Struct {
                variant: PrivVariantMeta::Tuple(tuple),
                ..
            } if tuple.args == 0 => {
                named.assert_not_generic()?;
                c.asm.push_with_comment(
                    Inst::Call {
                        hash: tuple.hash,
                        args: 0,
                    },
                    span,
                    meta.info().to_string(),
                );
            }
            PrivMetaKind::Struct {
                variant: PrivVariantMeta::Tuple(tuple),
                ..
            } => {
                named.assert_not_generic()?;
                c.asm.push_with_comment(
                    Inst::LoadFn { hash: tuple.hash },
                    span,
                    meta.info().to_string(),
                );
            }
            PrivMetaKind::Variant {
                variant: PrivVariantMeta::Tuple(tuple),
                ..
            } => {
                named.assert_not_generic()?;
                c.asm.push_with_comment(
                    Inst::LoadFn { hash: tuple.hash },
                    span,
                    meta.info().to_string(),
                );
            }
            PrivMetaKind::Function { type_hash, .. } => {
                let hash = if let Some(generics) = named.generics {
                    let parameters = generics_parameters(generics, c)?;
                    type_hash.with_parameters(parameters)
                } else {
                    *type_hash
                };

                c.asm
                    .push_with_comment(Inst::LoadFn { hash }, span, meta.info().to_string());
            }
            PrivMetaKind::Const { const_value, .. } => {
                named.assert_not_generic()?;
                const_(span, c, const_value, Needs::Value)?;
            }
            _ => {
                return Err(CompileError::expected_meta(
                    span,
                    meta.info(),
                    "something that can be used as a value",
                ));
            }
        }
    } else {
        named.assert_not_generic()?;

        let type_hash = meta.type_hash_of().ok_or_else(|| {
            CompileError::expected_meta(span, meta.info(), "something that has a type")
        })?;

        c.asm.push(
            Inst::Push {
                value: InstValue::Type(type_hash),
            },
            span,
        );
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(())
}

/// Assemble a return statement from the given Assemble.
fn return_<T>(
    c: &mut Assembler<'_>,
    span: Span,
    ast: &T,
    asm: impl FnOnce(&T, &mut Assembler<'_>, Needs) -> CompileResult<Asm>,
) -> CompileResult<()> {
    let clean = c.scopes.total_var_count(span)?;

    let address = asm(ast, c, Needs::Value)?.apply_targeted(c)?;
    c.asm.push(Inst::Return { address, clean }, span);

    // Top address produces an anonymous variable, which is consumed by the
    // return statement.
    if let InstAddress::Top = address {
        c.scopes.undecl_anon(span, 1)?;
    }

    Ok(())
}

/// Compile a pattern based on the given offset.
#[instrument]
fn pat_with_offset(ast: &ast::Pat, c: &mut Assembler<'_>, offset: usize) -> CompileResult<()> {
    let span = ast.span();

    let load = |c: &mut Assembler, needs: Needs| {
        if needs.value() {
            c.asm.push(Inst::Copy { offset }, span);
        }

        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(ast, c, false_label, &load)? {
        c.diagnostics
            .let_pattern_might_panic(c.source_id, span, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(ok_label, span);
        c.asm.label(false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            span,
        );

        c.asm.label(ok_label)?;
    }

    Ok(())
}

/// Encode a pattern.
///
/// Patterns will clean up their own locals and execute a jump to `false_label`
/// in case the pattern does not match.
///
/// Returns a boolean indicating if the label was used.
#[instrument]
fn pat(
    ast: &ast::Pat,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<bool> {
    match ast {
        ast::Pat::PatPath(p) => {
            let span = p.span();

            let named = c.convert_path(&p.path)?;
            named.assert_not_generic()?;

            if let Some(meta) = c.try_lookup_meta(span, &named.item)? {
                if pat_meta_binding(span, c, &meta, false_label, load)? {
                    return Ok(true);
                }
            }

            if let Some(ident) = named.as_local() {
                load(c, Needs::Value)?;
                c.scopes.decl_var(ident, span)?;
                return Ok(false);
            }

            Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinding,
            ))
        }
        ast::Pat::PatIgnore(..) => {
            // ignore binding, but might still have side effects, so must
            // call the load generator.
            load(c, Needs::None)?;
            Ok(false)
        }
        ast::Pat::PatLit(p) => Ok(pat_lit(p, c, false_label, load)?),
        ast::Pat::PatVec(p) => {
            pat_vec(p, c, false_label, &load)?;
            Ok(true)
        }
        ast::Pat::PatTuple(p) => {
            pat_tuple(p, c, false_label, &load)?;
            Ok(true)
        }
        ast::Pat::PatObject(object) => {
            pat_object(object, c, false_label, &load)?;
            Ok(true)
        }
        pat => Err(CompileError::new(
            pat,
            CompileErrorKind::UnsupportedPatternExpr,
        )),
    }
}

/// Assemble a pattern literal.
#[instrument]
fn pat_lit(
    pat_lit: &ast::PatLit,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<bool> {
    loop {
        match &*pat_lit.expr {
            ast::Expr::Unary(ast::ExprUnary {
                op: ast::UnOp::Neg(..),
                expr,
                ..
            }) => {
                if let ast::Expr::Lit(ast::ExprLit {
                    lit: ast::Lit::Number(lit_number),
                    ..
                }) = &**expr
                {
                    let span = lit_number.span();
                    let integer = lit_number
                        .resolve(resolve_context!(c.q))?
                        .as_i64(pat_lit.span(), true)?;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqInteger { integer }, span);
                    break;
                }
            }
            ast::Expr::Lit(expr_lit) => match &expr_lit.lit {
                ast::Lit::Byte(lit_byte) => {
                    let byte = lit_byte.resolve(resolve_context!(c.q))?;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqByte { byte }, lit_byte.span());
                    break;
                }
                ast::Lit::Char(lit_char) => {
                    let character = lit_char.resolve(resolve_context!(c.q))?;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqCharacter { character }, lit_char.span());
                    break;
                }
                ast::Lit::Str(pat_string) => {
                    let span = pat_string.span();
                    let string = pat_string.resolve(resolve_context!(c.q))?;
                    let slot = c.q.unit.new_static_string(span, &*string)?;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqStaticString { slot }, span);
                    break;
                }
                ast::Lit::Number(lit_number) => {
                    let span = lit_number.span();
                    let integer = lit_number
                        .resolve(resolve_context!(c.q))?
                        .as_i64(pat_lit.span(), false)?;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqInteger { integer }, span);
                    break;
                }
                ast::Lit::Bool(lit_bool) => {
                    let span = lit_bool.span();
                    let boolean = lit_bool.value;
                    load(c, Needs::Value)?;
                    c.asm.push(Inst::EqBool { boolean }, span);
                    break;
                }
                ast::Lit::ByteStr(_) => {}
            },
            _ => (),
        }

        return Err(CompileError::new(
            pat_lit,
            CompileErrorKind::UnsupportedPatternExpr,
        ));
    }

    let span = pat_lit.span();
    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);
    Ok(true)
}

/// Assemble an [ast::Condition].
#[instrument]
fn condition(
    condition: &ast::Condition,
    c: &mut Assembler<'_>,
    then_label: Label,
) -> CompileResult<Scope> {
    match condition {
        ast::Condition::Expr(e) => {
            let span = e.span();

            expr(e, c, Needs::Value)?.apply(c)?;
            c.asm.jump_if(then_label, span);

            Ok(c.scopes.child(span)?)
        }
        ast::Condition::ExprLet(expr_let) => {
            let span = expr_let.span();

            let false_label = c.asm.new_label("if_condition_false");

            let scope = c.scopes.child(span)?;
            let expected = c.scopes.push(scope);

            let load = |c: &mut Assembler<'_>, needs: Needs| {
                expr(&expr_let.expr, c, needs)?.apply(c)?;
                Ok(())
            };

            if pat(&expr_let.pat, c, false_label, &load)? {
                c.asm.jump(then_label, span);
                c.asm.label(false_label)?;
            } else {
                c.asm.jump(then_label, span);
            };

            let scope = c.scopes.pop(expected, span)?;
            Ok(scope)
        }
    }
}

/// Encode a vector pattern match.
#[instrument]
fn pat_vec(
    ast: &ast::PatVec,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    let span = ast.span();

    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    load(c, Needs::Value)?;
    let offset = c.scopes.decl_anon(span)?;

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    c.asm.push(Inst::Copy { offset }, span);

    let (is_open, count) = pat_items_count(&ast.items)?;

    c.asm.push(
        Inst::MatchSequence {
            type_check: TypeCheck::Vec,
            len: count,
            exact: !is_open,
        },
        span,
    );

    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);

    for (index, (p, _)) in ast.items.iter().take(count).enumerate() {
        let span = p.span();

        let load = move |c: &mut Assembler<'_>, needs: Needs| {
            if needs.value() {
                c.asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            }

            Ok(())
        };

        pat(p, c, false_label, &load)?;
    }

    Ok(())
}

/// Construct the appropriate match instruction for the given [PrivMeta].
#[instrument]
fn struct_match_for<'a>(
    span: Span,
    c: &Assembler<'_>,
    meta: &'a PrivMeta,
) -> Option<(&'a PrivStructMeta, Inst)> {
    Some(match &meta.kind {
        PrivMetaKind::Struct {
            type_hash,
            variant: PrivVariantMeta::Struct(st),
            ..
        } => (st, Inst::MatchType { hash: *type_hash }),
        PrivMetaKind::Variant {
            type_hash,
            enum_hash,
            index,
            variant: PrivVariantMeta::Struct(st),
            ..
        } => {
            let inst = if let Some(type_check) = c.context.type_check_for(*type_hash) {
                Inst::MatchBuiltIn { type_check }
            } else {
                Inst::MatchVariant {
                    variant_hash: *type_hash,
                    enum_hash: *enum_hash,
                    index: *index,
                }
            };

            (st, inst)
        }
        _ => {
            return None;
        }
    })
}

/// Construct the appropriate match instruction for the given [PrivMeta].
#[instrument]
fn tuple_match_for(span: Span, c: &Assembler<'_>, meta: &PrivMeta) -> Option<(usize, Inst)> {
    Some(match &meta.kind {
        PrivMetaKind::Struct {
            type_hash,
            variant: PrivVariantMeta::Unit,
            ..
        } => (0, Inst::MatchType { hash: *type_hash }),
        PrivMetaKind::Struct {
            type_hash,
            variant: PrivVariantMeta::Tuple(tuple),
            ..
        } => (tuple.args, Inst::MatchType { hash: *type_hash }),
        PrivMetaKind::Variant {
            enum_hash,
            type_hash,
            index,
            variant,
            ..
        } => {
            let args = match variant {
                PrivVariantMeta::Tuple(tuple) => tuple.args,
                PrivVariantMeta::Unit => 0,
                _ => return None,
            };

            let inst = if let Some(type_check) = c.context.type_check_for(*type_hash) {
                Inst::MatchBuiltIn { type_check }
            } else {
                Inst::MatchVariant {
                    enum_hash: *enum_hash,
                    variant_hash: *type_hash,
                    index: *index,
                }
            };

            (args, inst)
        }
        _ => return None,
    })
}

/// Encode a vector pattern match.
#[instrument]
fn pat_tuple(
    ast: &ast::PatTuple,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    let span = ast.span();

    load(c, Needs::Value)?;

    if ast.items.is_empty() {
        c.asm.push(Inst::IsUnit, span);

        c.asm
            .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);
        return Ok(());
    }

    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    let offset = c.scopes.decl_anon(span)?;

    let (is_open, count) = pat_items_count(&ast.items)?;

    if let Some(path) = &ast.path {
        let named = c.convert_path(path)?;
        named.assert_not_generic()?;

        let meta = c.lookup_meta(path.span(), &named.item)?;

        // Treat the current meta as a tuple and get the number of arguments it
        // should receive and the type check that applies to it.
        let (args, inst) = match tuple_match_for(span, c, &meta) {
            Some(out) => out,
            None => {
                return Err(CompileError::expected_meta(
                    span,
                    meta.info(),
                    "type that can be used in a tuple pattern",
                ));
            }
        };

        if !(args == count || count < args && is_open) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedArgumentCount {
                    meta: meta.info(),
                    expected: args,
                    actual: count,
                },
            ));
        }

        c.asm.push(Inst::Copy { offset }, span);
        c.asm.push(inst, span);
    } else {
        c.asm.push(Inst::Copy { offset }, span);
        c.asm.push(
            Inst::MatchSequence {
                type_check: TypeCheck::Tuple,
                len: count,
                exact: !is_open,
            },
            span,
        );
    };

    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);

    for (index, (p, _)) in ast.items.iter().take(count).enumerate() {
        let span = p.span();

        let load = move |c: &mut Assembler<'_>, needs: Needs| {
            if needs.value() {
                c.asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            }

            Ok(())
        };

        pat(p, c, false_label, &load)?;
    }

    Ok(())
}

/// Assemble an object pattern.
#[instrument]
fn pat_object(
    ast: &ast::PatObject,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    let span = ast.span();

    // NB: bind the loaded variable (once) to an anonymous var.
    // We reduce the number of copy operations by having specialized
    // operations perform the load from the given offset.
    load(c, Needs::Value)?;
    let offset = c.scopes.decl_anon(span)?;

    let mut string_slots = Vec::new();

    let mut keys_dup = HashMap::new();
    let mut keys = Vec::new();

    let mut bindings = Vec::new();
    let (has_rest, count) = pat_items_count(&ast.items)?;

    for (pat, _) in ast.items.iter().take(count) {
        let span = pat.span();
        let cow_key;

        let key = match pat {
            ast::Pat::PatBinding(binding) => {
                cow_key = binding.key.resolve(resolve_context!(c.q))?;
                bindings.push(Binding::Binding(
                    binding.span(),
                    cow_key.as_ref().into(),
                    &*binding.pat,
                ));
                cow_key.as_ref()
            }
            ast::Pat::PatPath(path) => {
                let ident = match path.path.try_as_ident() {
                    Some(ident) => ident,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedPatternExpr,
                        ));
                    }
                };

                let key = ident.resolve(resolve_context!(c.q))?;
                bindings.push(Binding::Ident(path.span(), key.into()));
                key
            }
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedPatternExpr,
                ));
            }
        };

        string_slots.push(c.q.unit.new_static_string(span, key)?);

        if let Some(existing) = keys_dup.insert(key.to_string(), span) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::DuplicateObjectKey {
                    existing,
                    object: ast.span(),
                },
            ));
        }

        keys.push(key.to_string());
    }

    match &ast.ident {
        ast::ObjectIdent::Named(path) => {
            let path_span = path.span();

            let named = c.convert_path(path)?;
            named.assert_not_generic()?;

            let meta = c.lookup_meta(path_span, &named.item)?;

            let (st, inst) = match struct_match_for(span, c, &meta) {
                Some(out) => out,
                None => {
                    return Err(CompileError::expected_meta(
                        path_span,
                        meta.info(),
                        "type that can be used in a struct pattern",
                    ));
                }
            };

            let mut fields = st.fields.clone();

            for binding in &bindings {
                if !fields.remove(binding.key()) {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::LitObjectNotField {
                            field: binding.key().into(),
                            item: meta.item.item.clone(),
                        },
                    ));
                }
            }

            if !has_rest && !fields.is_empty() {
                let mut fields = fields
                    .into_iter()
                    .map(Box::<str>::from)
                    .collect::<Box<[_]>>();
                fields.sort();

                return Err(CompileError::new(
                    ast.items.span(),
                    CompileErrorKind::PatternMissingFields {
                        item: meta.item.item.clone(),
                        fields,
                    },
                ));
            }

            c.asm.push(Inst::Copy { offset }, span);
            c.asm.push(inst, span);
        }
        ast::ObjectIdent::Anonymous(..) => {
            let keys = c.q.unit.new_static_object_keys_iter(span, &keys[..])?;

            // Copy the temporary and check that its length matches the pattern and
            // that it is indeed a vector.
            c.asm.push(Inst::Copy { offset }, span);
            c.asm.push(
                Inst::MatchObject {
                    slot: keys,
                    exact: !has_rest,
                },
                span,
            );
        }
    }

    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);

    for (binding, slot) in bindings.iter().zip(string_slots) {
        let span = binding.span();

        match binding {
            Binding::Binding(_, _, p) => {
                let load = move |c: &mut Assembler<'_>, needs: Needs| {
                    if needs.value() {
                        c.asm.push(Inst::ObjectIndexGetAt { offset, slot }, span);
                    }

                    Ok(())
                };

                pat(p, c, false_label, &load)?;
            }
            Binding::Ident(_, key) => {
                c.asm.push(Inst::ObjectIndexGetAt { offset, slot }, span);
                c.scopes.decl_var(key, span)?;
            }
        }
    }

    return Ok(());

    enum Binding<'a> {
        Binding(Span, Box<str>, &'a ast::Pat),
        Ident(Span, Box<str>),
    }

    impl Binding<'_> {
        fn span(&self) -> Span {
            match self {
                Self::Binding(span, _, _) => *span,
                Self::Ident(span, _) => *span,
            }
        }

        fn key(&self) -> &str {
            match self {
                Self::Binding(_, key, _) => key.as_ref(),
                Self::Ident(_, key) => key.as_ref(),
            }
        }
    }
}

/// Compile a binding name that matches a known meta type.
///
/// Returns `true` if the binding was used.
#[instrument]
fn pat_meta_binding(
    span: Span,
    c: &mut Assembler<'_>,
    meta: &PrivMeta,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<bool> {
    let inst = match tuple_match_for(span, c, meta) {
        Some((args, inst)) if args == 0 => inst,
        _ => return Ok(false),
    };

    load(c, Needs::Value)?;
    c.asm.push(inst, span);
    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);
    Ok(true)
}

/// Assemble an async block.
#[instrument]
pub(crate) fn closure_from_block(
    ast: &ast::Block,
    c: &mut Assembler<'_>,
    captures: &[CaptureMeta],
) -> CompileResult<()> {
    let span = ast.span();

    let guard = c.scopes.push_child(span)?;

    for capture in captures {
        c.scopes.new_var(&capture.ident, span)?;
    }

    return_(c, span, ast, block)?;
    c.scopes.pop(guard, span)?;
    Ok(())
}

/// Call a block.
#[instrument]
fn block(ast: &ast::Block, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    c.contexts.push(span);
    let scopes_count = c.scopes.push_child(span)?;

    let mut last = None::<(&ast::Expr, bool)>;

    for stmt in &ast.statements {
        let (e, term) = match stmt {
            ast::Stmt::Local(l) => {
                if let Some((e, _)) = std::mem::take(&mut last) {
                    // NB: terminated expressions do not need to produce a value.
                    expr(e, c, Needs::None)?.apply(c)?;
                }

                local(l, c, Needs::None)?.apply(c)?;
                continue;
            }
            ast::Stmt::Expr(expr, semi) => (expr, semi.is_some()),
            ast::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = std::mem::replace(&mut last, Some((e, term))) {
            // NB: terminated expressions do not need to produce a value.
            expr(e, c, Needs::None)?.apply(c)?;
        }
    }

    let produced = if let Some((e, term)) = last {
        if term {
            expr(e, c, Needs::None)?.apply(c)?;
            false
        } else {
            expr(e, c, needs)?.apply(c)?;
            true
        }
    } else {
        false
    };

    let scope = c.scopes.pop(scopes_count, span)?;

    if needs.value() {
        if produced {
            c.locals_clean(scope.local_var_count, span);
        } else {
            c.locals_pop(scope.local_var_count, span);
            c.asm.push(Inst::unit(), span);
        }
    } else {
        c.locals_pop(scope.local_var_count, span);
    }

    c.contexts
        .pop()
        .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

    Ok(Asm::top(span))
}

/// Assemble #[builtin] format!(...) macro.
#[instrument]
fn builtin_format(ast: &BuiltInFormat, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    use crate::runtime::format;

    let span = ast.span();

    let fill = if let Some((_, fill)) = &ast.fill {
        *fill
    } else {
        ' '
    };

    let align = if let Some((_, align)) = &ast.align {
        *align
    } else {
        format::Alignment::default()
    };

    let flags = if let Some((_, flags)) = &ast.flags {
        *flags
    } else {
        format::Flags::default()
    };

    let width = if let Some((_, width)) = &ast.width {
        *width
    } else {
        None
    };

    let precision = if let Some((_, precision)) = &ast.precision {
        *precision
    } else {
        None
    };

    let format_type = if let Some((_, format_type)) = &ast.format_type {
        *format_type
    } else {
        format::Type::default()
    };

    let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

    expr(&ast.value, c, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Format { spec }, span);

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument]
fn builtin_template(
    ast: &BuiltInTemplate,
    c: &mut Assembler<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let span = ast.span();

    let expected = c.scopes.push_child(span)?;
    let mut size_hint = 0;
    let mut expansions = 0;

    for e in &ast.exprs {
        if let ast::Expr::Lit(ast::ExprLit {
            lit: ast::Lit::Str(s),
            ..
        }) = e
        {
            let s = s.resolve_template_string(resolve_context!(c.q))?;
            size_hint += s.len();

            let slot = c.q.unit.new_static_string(span, &s)?;
            c.asm.push(Inst::String { slot }, span);
            c.scopes.decl_anon(span)?;
            continue;
        }

        expansions += 1;
        expr(e, c, Needs::Value)?.apply(c)?;
        c.scopes.decl_anon(span)?;
    }

    if ast.from_literal && expansions == 0 {
        c.diagnostics
            .template_without_expansions(c.source_id, span, c.context());
    }

    c.asm.push(
        Inst::StringConcat {
            len: ast.exprs.len(),
            size_hint,
        },
        span,
    );

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    let _ = c.scopes.pop(expected, span)?;
    Ok(Asm::top(span))
}

/// Assemble a constant value.
#[instrument]
fn const_(
    span: Span,
    c: &mut Assembler<'_>,
    value: &ConstValue,
    needs: Needs,
) -> CompileResult<()> {
    use num::ToPrimitive;

    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(());
    }

    match value {
        ConstValue::Unit => {
            c.asm.push(Inst::unit(), span);
        }
        ConstValue::Byte(b) => {
            c.asm.push(Inst::byte(*b), span);
        }
        ConstValue::Char(ch) => {
            c.asm.push(Inst::char(*ch), span);
        }
        ConstValue::Integer(n) => {
            let n = match n.to_i64() {
                Some(n) => n,
                None => {
                    return Err(CompileError::new(
                        span,
                        ParseErrorKind::BadNumberOutOfBounds,
                    ));
                }
            };

            c.asm.push(Inst::integer(n), span);
        }
        ConstValue::Float(n) => {
            c.asm.push(Inst::float(*n), span);
        }
        ConstValue::Bool(b) => {
            c.asm.push(Inst::bool(*b), span);
        }
        ConstValue::String(s) => {
            let slot = c.q.unit.new_static_string(span, s)?;
            c.asm.push(Inst::String { slot }, span);
        }
        ConstValue::StaticString(s) => {
            let slot = c.q.unit.new_static_string(span, s.as_ref())?;
            c.asm.push(Inst::String { slot }, span);
        }
        ConstValue::Bytes(b) => {
            let slot = c.q.unit.new_static_bytes(span, &*b)?;
            c.asm.push(Inst::Bytes { slot }, span);
        }
        ConstValue::Option(option) => match option {
            Some(value) => {
                const_(span, c, value, Needs::Value)?;
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    span,
                );
            }
            None => {
                c.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                );
            }
        },
        ConstValue::Vec(vec) => {
            for value in vec.iter() {
                const_(span, c, value, Needs::Value)?;
            }

            c.asm.push(Inst::Vec { count: vec.len() }, span);
        }
        ConstValue::Tuple(tuple) => {
            for value in tuple.iter() {
                const_(span, c, value, Needs::Value)?;
            }

            c.asm.push(Inst::Tuple { count: tuple.len() }, span);
        }
        ConstValue::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|k| k.0);

            for (_, value) in entries.iter().copied() {
                const_(span, c, value, Needs::Value)?;
            }

            let slot =
                c.q.unit
                    .new_static_object_keys_iter(span, entries.iter().map(|e| e.0))?;

            c.asm.push(Inst::Object { slot }, span);
        }
    }

    Ok(())
}

/// Assemble an expression.
#[instrument]
fn expr(ast: &ast::Expr, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    use crate::query::BuiltInMacro;

    let asm = match ast {
        ast::Expr::Path(p) => path(p, c, needs)?,
        ast::Expr::While(e) => expr_while(e, c, needs)?,
        ast::Expr::For(e) => expr_for(e, c, needs)?,
        ast::Expr::Loop(e) => expr_loop(e, c, needs)?,
        ast::Expr::Let(e) => expr_let(e, c, needs)?,
        ast::Expr::Group(e) => expr(&e.expr, c, needs)?,
        ast::Expr::Empty(e) => expr(&e.expr, c, needs)?,
        ast::Expr::Unary(e) => expr_unary(e, c, needs)?,
        ast::Expr::Assign(e) => expr_assign(e, c, needs)?,
        ast::Expr::Binary(e) => expr_binary(e, c, needs)?,
        ast::Expr::If(e) => expr_if(e, c, needs)?,
        ast::Expr::Index(e) => expr_index(e, c, needs)?,
        ast::Expr::Break(e) => expr_break(e, c, needs)?,
        ast::Expr::Continue(e) => expr_continue(e, c, needs)?,
        ast::Expr::Yield(e) => expr_yield(e, c, needs)?,
        ast::Expr::Block(e) => expr_block(e, c, needs)?,
        ast::Expr::Return(e) => expr_return(e, c, needs)?,
        ast::Expr::Match(e) => expr_match(e, c, needs)?,
        ast::Expr::Await(e) => expr_await(e, c, needs)?,
        ast::Expr::Try(e) => expr_try(e, c, needs)?,
        ast::Expr::Select(e) => expr_select(e, c, needs)?,
        ast::Expr::Call(e) => expr_call(e, c, needs)?,
        ast::Expr::FieldAccess(e) => expr_field_access(e, c, needs)?,
        ast::Expr::Closure(e) => expr_closure(e, c, needs)?,
        ast::Expr::Lit(e) => lit(&e.lit, c, needs)?,
        ast::Expr::ForceSemi(e) => expr(&e.expr, c, needs)?,
        ast::Expr::Tuple(e) => expr_tuple(e, c, needs)?,
        ast::Expr::Vec(e) => expr_vec(e, c, needs)?,
        ast::Expr::Object(e) => expr_object(e, c, needs)?,
        ast::Expr::Range(e) => expr_range(e, c, needs)?,
        ast::Expr::MacroCall(expr_call_macro) => {
            let internal_macro = c.q.builtin_macro_for(expr_call_macro)?;

            match &*internal_macro {
                BuiltInMacro::Template(template) => builtin_template(template, c, needs)?,
                BuiltInMacro::Format(format) => builtin_format(format, c, needs)?,
                BuiltInMacro::Line(line) => lit_number(&line.value, c, needs)?,
                BuiltInMacro::File(file) => lit_str(&file.value, c, needs)?,
            }
        }
    };

    Ok(asm)
}

/// Assemble an assign expression.
#[instrument]
fn expr_assign(ast: &ast::ExprAssign, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let supported = match &*ast.lhs {
        // <var> = <value>
        ast::Expr::Path(path) if path.rest.is_empty() => {
            expr(&ast.rhs, c, Needs::Value)?.apply(c)?;

            let segment = path
                .first
                .try_as_ident()
                .ok_or_else(|| CompileError::msg(path, "unsupported path"))?;
            let ident = segment.resolve(resolve_context!(c.q))?;
            let var = c.scopes.get_var(c.q.visitor, &*ident, c.source_id, span)?;
            c.asm.push(Inst::Replace { offset: var.offset }, span);
            true
        }
        // <expr>.<field> = <value>
        ast::Expr::FieldAccess(field_access) => {
            let span = field_access.span();

            // field assignment
            match &field_access.expr_field {
                ast::ExprField::Path(path) => {
                    if let Some(ident) = path.try_as_ident() {
                        let slot = ident.resolve(resolve_context!(c.q))?;
                        let slot = c.q.unit.new_static_string(ident.span(), slot.as_ref())?;

                        expr(&ast.rhs, c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(ast.rhs.span())?;

                        expr(&field_access.expr, c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(span)?;

                        c.asm.push(Inst::ObjectIndexSet { slot }, span);
                        c.scopes.undecl_anon(span, 2)?;
                        true
                    } else {
                        false
                    }
                }
                ast::ExprField::LitNumber(field) => {
                    let number = field.resolve(resolve_context!(c.q))?;
                    let index = number.as_tuple_index().ok_or_else(|| {
                        CompileError::new(span, CompileErrorKind::UnsupportedTupleIndex { number })
                    })?;

                    expr(&ast.rhs, c, Needs::Value)?.apply(c)?;
                    c.scopes.decl_anon(ast.rhs.span())?;

                    expr(&field_access.expr, c, Needs::Value)?.apply(c)?;
                    c.asm.push(Inst::TupleIndexSet { index }, span);
                    c.scopes.undecl_anon(span, 1)?;
                    true
                }
            }
        }
        ast::Expr::Index(expr_index_get) => {
            let span = expr_index_get.span();

            expr(&ast.rhs, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            expr(&expr_index_get.target, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            expr(&expr_index_get.index, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            c.asm.push(Inst::IndexSet, span);
            c.scopes.undecl_anon(span, 3)?;
            true
        }
        _ => false,
    };

    if !supported {
        return Err(CompileError::new(
            span,
            CompileErrorKind::UnsupportedAssignExpr,
        ));
    }

    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    Ok(Asm::top(span))
}

/// Assemble an `.await` expression.
#[instrument]
fn expr_await(ast: &ast::ExprAwait, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    expr(&ast.expr, c, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Await, span);

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a binary expression.
#[instrument]
fn expr_binary(ast: &ast::ExprBinary, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    // Special expressions which operates on the stack in special ways.
    if ast.op.is_assign() {
        compile_assign_binop(c, &ast.lhs, &ast.rhs, &ast.op, needs)?;
        return Ok(Asm::top(span));
    }

    if ast.op.is_conditional() {
        compile_conditional_binop(c, &ast.lhs, &ast.rhs, &ast.op, needs)?;
        return Ok(Asm::top(span));
    }

    let guard = c.scopes.push_child(span)?;

    // NB: need to declare these as anonymous local variables so that they
    // get cleaned up in case there is an early break (return, try, ...).
    let rhs_needs = rhs_needs_of(&ast.op);
    let a = expr(&ast.lhs, c, Needs::Value)?.apply_targeted(c)?;
    let b = expr(&ast.rhs, c, rhs_needs)?.apply_targeted(c)?;

    let op = match ast.op {
        ast::BinOp::Eq(..) => InstOp::Eq,
        ast::BinOp::Neq(..) => InstOp::Neq,
        ast::BinOp::Lt(..) => InstOp::Lt,
        ast::BinOp::Gt(..) => InstOp::Gt,
        ast::BinOp::Lte(..) => InstOp::Lte,
        ast::BinOp::Gte(..) => InstOp::Gte,
        ast::BinOp::Is(..) => InstOp::Is,
        ast::BinOp::IsNot(..) => InstOp::IsNot,
        ast::BinOp::And(..) => InstOp::And,
        ast::BinOp::Or(..) => InstOp::Or,
        ast::BinOp::Add(..) => InstOp::Add,
        ast::BinOp::Sub(..) => InstOp::Sub,
        ast::BinOp::Div(..) => InstOp::Div,
        ast::BinOp::Mul(..) => InstOp::Mul,
        ast::BinOp::Rem(..) => InstOp::Rem,
        ast::BinOp::BitAnd(..) => InstOp::BitAnd,
        ast::BinOp::BitXor(..) => InstOp::BitXor,
        ast::BinOp::BitOr(..) => InstOp::BitOr,
        ast::BinOp::Shl(..) => InstOp::Shl,
        ast::BinOp::Shr(..) => InstOp::Shr,

        op => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedBinaryOp { op },
            ));
        }
    };

    c.asm.push(Inst::Op { op, a, b }, span);

    // NB: we put it here to preserve the call in case it has side effects.
    // But if we don't need the value, then pop it from the stack.
    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    c.scopes.pop(guard, span)?;
    return Ok(Asm::top(span));

    /// Get the need of the right-hand side operator from the type of the
    /// operator.
    fn rhs_needs_of(op: &ast::BinOp) -> Needs {
        match op {
            ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
            _ => Needs::Value,
        }
    }

    fn compile_conditional_binop(
        c: &mut Assembler<'_>,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        bin_op: &ast::BinOp,
        needs: Needs,
    ) -> CompileResult<()> {
        let span = lhs.span().join(rhs.span());

        let end_label = c.asm.new_label("conditional_end");

        expr(lhs, c, Needs::Value)?.apply(c)?;

        match bin_op {
            ast::BinOp::And(..) => {
                c.asm.jump_if_not_or_pop(end_label, lhs.span());
            }
            ast::BinOp::Or(..) => {
                c.asm.jump_if_or_pop(end_label, lhs.span());
            }
            op => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryOp { op: *op },
                ));
            }
        }

        expr(rhs, c, Needs::Value)?.apply(c)?;

        c.asm.label(end_label)?;

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_assign_binop(
        c: &mut Assembler<'_>,
        lhs: &ast::Expr,
        rhs: &ast::Expr,
        bin_op: &ast::BinOp,
        needs: Needs,
    ) -> CompileResult<()> {
        let span = lhs.span().join(rhs.span());

        let supported = match lhs {
            // <var> <op> <expr>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                expr(rhs, c, Needs::Value)?.apply(c)?;

                let segment = path
                    .first
                    .try_as_ident()
                    .ok_or_else(|| CompileError::msg(path, "unsupported path segment"))?;

                let ident = segment.resolve(resolve_context!(c.q))?;
                let var = c.scopes.get_var(c.q.visitor, &*ident, c.source_id, span)?;

                Some(InstTarget::Offset(var.offset))
            }
            // <expr>.<field> <op> <value>
            ast::Expr::FieldAccess(field_access) => {
                expr(&field_access.expr, c, Needs::Value)?.apply(c)?;
                expr(rhs, c, Needs::Value)?.apply(c)?;

                // field assignment
                match &field_access.expr_field {
                    ast::ExprField::Path(path) => {
                        if let Some(ident) = path.try_as_ident() {
                            let n = ident.resolve(resolve_context!(c.q))?;
                            let n = c.q.unit.new_static_string(path.span(), n.as_ref())?;

                            Some(InstTarget::Field(n))
                        } else {
                            None
                        }
                    }
                    ast::ExprField::LitNumber(field) => {
                        let span = field.span();

                        let number = field.resolve(resolve_context!(c.q))?;
                        let index = number.as_tuple_index().ok_or_else(|| {
                            CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            )
                        })?;

                        Some(InstTarget::TupleField(index))
                    }
                }
            }
            _ => None,
        };

        let target = match supported {
            Some(target) => target,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        let op = match bin_op {
            ast::BinOp::AddAssign(..) => InstAssignOp::Add,
            ast::BinOp::SubAssign(..) => InstAssignOp::Sub,
            ast::BinOp::MulAssign(..) => InstAssignOp::Mul,
            ast::BinOp::DivAssign(..) => InstAssignOp::Div,
            ast::BinOp::RemAssign(..) => InstAssignOp::Rem,
            ast::BinOp::BitAndAssign(..) => InstAssignOp::BitAnd,
            ast::BinOp::BitXorAssign(..) => InstAssignOp::BitXor,
            ast::BinOp::BitOrAssign(..) => InstAssignOp::BitOr,
            ast::BinOp::ShlAssign(..) => InstAssignOp::Shl,
            ast::BinOp::ShrAssign(..) => InstAssignOp::Shr,
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryExpr,
                ));
            }
        };

        c.asm.push(Inst::Assign { target, op }, span);

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        Ok(())
    }
}

/// Assemble a block expression.
#[instrument]
fn expr_block(ast: &ast::ExprBlock, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    if ast.async_token.is_none() && ast.const_token.is_none() {
        return block(&ast.block, c, needs);
    }

    let item = c.q.item_for(&ast.block)?;
    let meta = c.lookup_meta(span, &item.item)?;

    match &meta.kind {
        PrivMetaKind::AsyncBlock {
            captures, do_move, ..
        } => {
            let captures = &**captures;
            let do_move = *do_move;

            for ident in captures {
                if do_move {
                    let var = c
                        .scopes
                        .take_var(c.q.visitor, &ident.ident, c.source_id, span)?;
                    var.do_move(c.asm, span, format!("captures `{}`", ident.ident));
                } else {
                    let var = c
                        .scopes
                        .get_var(c.q.visitor, &ident.ident, c.source_id, span)?;
                    var.copy(c, span, format!("captures `{}`", ident.ident));
                }
            }

            let hash = Hash::type_hash(&meta.item.item);
            c.asm.push_with_comment(
                Inst::Call {
                    hash,
                    args: captures.len(),
                },
                span,
                meta.info().to_string(),
            );

            if !needs.value() {
                c.asm
                    .push_with_comment(Inst::Pop, span, "value is not needed");
            }
        }
        PrivMetaKind::Const { const_value } => {
            const_(span, c, const_value, needs)?;
        }
        _ => {
            return Err(CompileError::expected_meta(
                span,
                meta.info(),
                "async block",
            ));
        }
    };

    Ok(Asm::top(span))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument]
fn expr_break(ast: &ast::ExprBreak, c: &mut Assembler<'_>, _: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let current_loop = match c.loops.last() {
        Some(current_loop) => current_loop,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::BreakOutsideOfLoop,
            ));
        }
    };

    let (last_loop, to_drop, has_value) = if let Some(e) = ast.expr.as_deref() {
        match e {
            ast::ExprBreakValue::Expr(e) => {
                expr(e, c, current_loop.needs)?.apply(c)?;
                (current_loop, current_loop.drop.into_iter().collect(), true)
            }
            ast::ExprBreakValue::Label(label) => {
                let (last_loop, to_drop) =
                    c.loops.walk_until_label(resolve_context!(c.q), label)?;
                (last_loop, to_drop, false)
            }
        }
    } else {
        (current_loop, current_loop.drop.into_iter().collect(), false)
    };

    // Drop loop temporary. Typically an iterator.
    for offset in to_drop {
        c.asm.push(Inst::Drop { offset }, span);
    }

    let vars = c
        .scopes
        .total_var_count(span)?
        .checked_sub(last_loop.break_var_count)
        .ok_or_else(|| CompileError::msg(&span, "var count should be larger"))?;

    if last_loop.needs.value() {
        if has_value {
            c.locals_clean(vars, span);
        } else {
            c.locals_pop(vars, span);
            c.asm.push(Inst::unit(), span);
        }
    } else {
        c.locals_pop(vars, span);
    }

    c.asm.jump(last_loop.break_label, span);
    Ok(Asm::top(span))
}

fn generics_parameters(
    generics: &ast::AngleBracketed<ast::PathSegmentExpr, T![,]>,
    c: &mut Assembler<'_>,
) -> Result<Hash, CompileError> {
    let mut parameters = ParametersBuilder::new();

    for (param, _) in generics {
        let path = match &param.expr {
            ast::Expr::Path(path) => path,
            e => {
                return Err(CompileError::new(e, CompileErrorKind::UnsupportedGenerics));
            }
        };

        let named = c.convert_path(path)?;
        named.assert_not_generic()?;

        let meta = c.lookup_meta(param.span(), &named.item)?;

        let hash = match meta.kind {
            PrivMetaKind::Unknown { type_hash, .. } => type_hash,
            PrivMetaKind::Struct { type_hash, .. } => type_hash,
            PrivMetaKind::Enum { type_hash, .. } => type_hash,
            _ => {
                return Err(CompileError::new(
                    generics,
                    CompileErrorKind::UnsupportedGenerics,
                ));
            }
        };

        parameters.add(hash);
    }

    Ok(parameters.finish())
}

enum Call {
    Var {
        /// The variable slot being called.
        var: Var,
        /// The name of the variable being called.
        name: Box<str>,
    },
    Instance {
        /// Hash of the fn being called.
        hash: Hash,
    },
    Meta {
        /// PrivMeta being called.
        meta: PrivMeta,
        /// The hash of the meta thing being called.
        hash: Hash,
    },
    /// An expression being called.
    Expr,
    /// A constant function call.
    ConstFn {
        /// PrivMeta of the constand function.
        meta: PrivMeta,
        /// The identifier of the constant function.
        id: Id,
    },
}

/// Convert into a call expression.
fn convert_expr_call(ast: &ast::ExprCall, c: &mut Assembler<'_>) -> CompileResult<Call> {
    let span = ast.span();

    match &*ast.expr {
        ast::Expr::Path(path) => {
            let named = c.convert_path(path)?;

            if let Some(name) = named.as_local() {
                let local = c
                    .scopes
                    .try_get_var(c.q.visitor, name, c.source_id, path.span())?;

                if let Some(var) = local {
                    return Ok(Call::Var {
                        var,
                        name: name.into(),
                    });
                }
            }

            let meta = c.lookup_meta(path.span(), &named.item)?;
            debug_assert_eq!(meta.item.item, named.item);

            match &meta.kind {
                PrivMetaKind::Struct {
                    variant: PrivVariantMeta::Unit,
                    ..
                }
                | PrivMetaKind::Variant {
                    variant: PrivVariantMeta::Unit,
                    ..
                } => {
                    named.assert_not_generic()?;

                    if !ast.args.is_empty() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedArgumentCount {
                                meta: meta.info(),
                                expected: 0,
                                actual: ast.args.len(),
                            },
                        ));
                    }
                }
                PrivMetaKind::Struct {
                    variant: PrivVariantMeta::Tuple(tuple),
                    ..
                }
                | PrivMetaKind::Variant {
                    variant: PrivVariantMeta::Tuple(tuple),
                    ..
                } => {
                    named.assert_not_generic()?;

                    if tuple.args != ast.args.len() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedArgumentCount {
                                meta: meta.info(),
                                expected: tuple.args,
                                actual: ast.args.len(),
                            },
                        ));
                    }

                    if tuple.args == 0 {
                        let tuple = path.span();
                        c.diagnostics.remove_tuple_call_parens(
                            c.source_id,
                            span,
                            tuple,
                            c.context(),
                        );
                    }
                }
                PrivMetaKind::Function { .. } => (),
                PrivMetaKind::ConstFn { id, .. } => {
                    named.assert_not_generic()?;
                    let id = *id;
                    return Ok(Call::ConstFn { meta, id });
                }
                _ => {
                    return Err(CompileError::expected_meta(
                        span,
                        meta.info(),
                        "something that can be called as a function",
                    ));
                }
            };

            let hash = Hash::type_hash(&meta.item.item);

            let hash = if let Some(generics) = named.generics {
                let parameters = generics_parameters(generics, c)?;
                hash.with_parameters(parameters)
            } else {
                hash
            };

            return Ok(Call::Meta { meta, hash });
        }
        ast::Expr::FieldAccess(ast::ExprFieldAccess {
            expr_field: ast::ExprField::Path(path),
            ..
        }) => {
            if let Some((ident, generics)) = path.try_as_ident_generics() {
                let ident = ident.resolve(resolve_context!(c.q))?;
                let hash = Hash::instance_fn_name(ident);

                let hash = if let Some(generics) = generics {
                    let parameters = generics_parameters(generics, c)?;
                    hash.with_parameters(parameters)
                } else {
                    hash
                };

                return Ok(Call::Instance { hash });
            }
        }
        _ => {}
    };

    Ok(Call::Expr)
}

/// Assemble a call expression.
#[instrument]
fn expr_call(ast: &ast::ExprCall, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let call = convert_expr_call(ast, c)?;

    let args = ast.args.len();

    match call {
        Call::Var { var, name } => {
            for (e, _) in &ast.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            var.copy(c, span, format!("var `{}`", name));
            c.scopes.decl_anon(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.undecl_anon(span, ast.args.len() + 1)?;
        }
        Call::Instance { hash } => {
            let target = ast.target();

            expr(target, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(target.span())?;

            for (e, _) in &ast.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            c.asm.push(Inst::CallInstance { hash, args }, span);
            c.scopes.undecl_anon(span, ast.args.len() + 1)?;
        }
        Call::Meta { meta, hash } => {
            for (e, _) in &ast.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            c.asm
                .push_with_comment(Inst::Call { hash, args }, span, meta.info().to_string());

            c.scopes.undecl_anon(span, args)?;
        }
        Call::Expr => {
            for (e, _) in &ast.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            expr(&ast.expr, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.undecl_anon(span, args + 1)?;
        }
        Call::ConstFn { meta, id } => {
            let from = c.q.item_for(ast)?;
            let const_fn = c.q.const_fn_for((ast.span(), id))?;
            let value = c.call_const_fn(ast, &meta, &from, &const_fn, ast.args.as_slice())?;
            const_(ast.span(), c, &value, Needs::Value)?;
        }
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble the body of a closure function.
#[instrument]
pub(crate) fn closure_from_expr_closure(
    ast: &ast::ExprClosure,
    c: &mut Assembler<'_>,
    captures: &[CaptureMeta],
) -> CompileResult<()> {
    let span = ast.span();

    let mut patterns = Vec::new();

    for (arg, _) in ast.args.as_slice() {
        match arg {
            ast::FnArg::SelfValue(s) => {
                return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf))
            }
            ast::FnArg::Pat(pat) => {
                let offset = c.scopes.decl_anon(pat.span())?;
                patterns.push((pat, offset));
            }
        }
    }

    if !captures.is_empty() {
        c.asm.push(Inst::PushTuple, span);

        for capture in captures {
            c.scopes.new_var(&capture.ident, span)?;
        }
    }

    for (pat, offset) in patterns {
        pat_with_offset(pat, c, offset)?;
    }

    return_(c, span, &*ast.body, expr)?;
    c.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a closure expression.
#[instrument]
fn expr_closure(ast: &ast::ExprClosure, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    let item = c.q.item_for(ast)?;
    let hash = Hash::type_hash(&item.item);

    let meta = match c.q.query_meta(span, &item.item, Default::default())? {
        Some(meta) => meta,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::MissingItem {
                    item: item.item.clone(),
                },
            ))
        }
    };

    let (captures, do_move) = match &meta.kind {
        PrivMetaKind::Closure {
            captures, do_move, ..
        } => (&**captures, *do_move),
        _ => {
            return Err(CompileError::expected_meta(span, meta.info(), "a closure"));
        }
    };

    tracing::trace!("captures: {} => {:?}", item.item, captures);

    if captures.is_empty() {
        // NB: if closure doesn't capture the environment it acts like a regular
        // function. No need to store and load the environment.
        c.asm.push_with_comment(
            Inst::LoadFn { hash },
            span,
            format!("closure `{}`", item.item),
        );
    } else {
        // Construct a closure environment.
        for capture in captures {
            if do_move {
                let var = c
                    .scopes
                    .take_var(c.q.visitor, &capture.ident, c.source_id, span)?;
                var.do_move(c.asm, span, format!("capture `{}`", capture.ident));
            } else {
                let var = c
                    .scopes
                    .get_var(c.q.visitor, &capture.ident, c.source_id, span)?;
                var.copy(c, span, format!("capture `{}`", capture.ident));
            }
        }

        c.asm.push_with_comment(
            Inst::Closure {
                hash,
                count: captures.len(),
            },
            span,
            format!("closure `{}`", item.item),
        );
    }

    Ok(Asm::top(span))
}

/// Assemble a continue expression.
#[instrument]
fn expr_continue(ast: &ast::ExprContinue, c: &mut Assembler<'_>, _: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let current_loop = match c.loops.last() {
        Some(current_loop) => current_loop,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ContinueOutsideOfLoop,
            ));
        }
    };

    let last_loop = if let Some(label) = &ast.label {
        let (last_loop, _) = c.loops.walk_until_label(resolve_context!(c.q), label)?;
        last_loop
    } else {
        current_loop
    };

    let vars = c
        .scopes
        .total_var_count(span)?
        .checked_sub(last_loop.continue_var_count)
        .ok_or_else(|| CompileError::msg(&span, "var count should be larger"))?;

    c.locals_pop(vars, span);

    c.asm.jump(last_loop.continue_label, span);
    Ok(Asm::top(span))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument]
fn expr_field_access(
    ast: &ast::ExprFieldAccess,
    c: &mut Assembler<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let span = ast.span();

    // Optimizations!
    //
    // TODO: perform deferred compilation for expressions instead, so we can
    // e.g. inspect if it compiles down to a local access instead of
    // climbing the ast like we do here.
    #[allow(clippy::single_match)]
    match (&*ast.expr, &ast.expr_field) {
        (ast::Expr::Path(path), ast::ExprField::LitNumber(n)) => {
            if try_immediate_field_access_optimization(c, span, path, n, needs)? {
                return Ok(Asm::top(span));
            }
        }
        _ => (),
    }

    expr(&ast.expr, c, Needs::Value)?.apply(c)?;

    match &ast.expr_field {
        ast::ExprField::LitNumber(n) => {
            if let Some(index) = n.resolve(resolve_context!(c.q))?.as_tuple_index() {
                c.asm.push(Inst::TupleIndexGet { index }, span);

                if !needs.value() {
                    c.diagnostics.not_used(c.source_id, span, c.context());
                    c.asm.push(Inst::Pop, span);
                }

                return Ok(Asm::top(span));
            }
        }
        ast::ExprField::Path(path) => {
            if let Some(ident) = path.try_as_ident() {
                let field = ident.resolve(resolve_context!(c.q))?;
                let slot = c.q.unit.new_static_string(span, field.as_ref())?;

                c.asm.push(Inst::ObjectIndexGet { slot }, span);

                if !needs.value() {
                    c.diagnostics.not_used(c.source_id, span, c.context());
                    c.asm.push(Inst::Pop, span);
                }

                return Ok(Asm::top(span));
            }
        }
    }

    return Err(CompileError::new(span, CompileErrorKind::BadFieldAccess));

    fn try_immediate_field_access_optimization(
        c: &mut Assembler<'_>,
        span: Span,
        path: &ast::Path,
        n: &ast::LitNumber,
        needs: Needs,
    ) -> CompileResult<bool> {
        let ident = match path.try_as_ident() {
            Some(ident) => ident,
            None => return Ok(false),
        };

        let ident = ident.resolve(resolve_context!(c.q))?;

        let index = match n.resolve(resolve_context!(c.q))? {
            ast::Number::Integer(n) => n,
            _ => return Ok(false),
        };

        let index = match usize::try_from(index) {
            Ok(index) => index,
            Err(..) => return Ok(false),
        };

        let var = match c
            .scopes
            .try_get_var(c.q.visitor, ident, c.source_id, path.span())?
        {
            Some(var) => var,
            None => return Ok(false),
        };

        c.asm.push(
            Inst::TupleIndexGetAt {
                offset: var.offset,
                index,
            },
            span,
        );

        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
        }

        Ok(true)
    }
}

/// Assemble an expression for loop.
#[instrument]
fn expr_for(ast: &ast::ExprFor, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let continue_label = c.asm.new_label("for_continue");
    let end_label = c.asm.new_label("for_end");
    let break_label = c.asm.new_label("for_break");

    let break_var_count = c.scopes.total_var_count(span)?;

    let (iter_offset, loop_scope_expected) = {
        let loop_scope_expected = c.scopes.push_child(span)?;
        expr(&ast.iter, c, Needs::Value)?.apply(c)?;

        let iter_offset = c.scopes.decl_anon(span)?;
        c.asm.push_with_comment(
            Inst::CallInstance {
                hash: *Protocol::INTO_ITER,
                args: 0,
            },
            span,
            format!("into_iter (offset: {})", iter_offset),
        );

        (iter_offset, loop_scope_expected)
    };

    let binding_span = ast.binding.span();

    // Declare named loop variable.
    let binding_offset = {
        c.asm.push(Inst::unit(), ast.iter.span());
        c.scopes.decl_anon(binding_span)?
    };

    // Declare storage for memoized `next` instance fn.
    let next_offset = if c.options.memoize_instance_fn {
        let span = ast.iter.span();

        let offset = c.scopes.decl_anon(span)?;

        // Declare the named loop variable and put it in the scope.
        c.asm.push_with_comment(
            Inst::Copy {
                offset: iter_offset,
            },
            span,
            "copy iterator (memoize)",
        );

        c.asm.push_with_comment(
            Inst::LoadInstanceFn {
                hash: *Protocol::NEXT,
            },
            span,
            "load instance fn (memoize)",
        );

        Some(offset)
    } else {
        None
    };

    let continue_var_count = c.scopes.total_var_count(span)?;
    c.asm.label(continue_label)?;

    let _guard = c.loops.push(Loop {
        label: ast.label.map(|(label, _)| label),
        continue_label,
        continue_var_count,
        break_label,
        break_var_count,
        needs,
        drop: Some(iter_offset),
    });

    // Use the memoized loop variable.
    if let Some(next_offset) = next_offset {
        c.asm.push_with_comment(
            Inst::Copy {
                offset: iter_offset,
            },
            ast.iter.span(),
            "copy iterator",
        );

        c.asm.push_with_comment(
            Inst::Copy {
                offset: next_offset,
            },
            ast.iter.span(),
            "copy next",
        );

        c.asm.push(Inst::CallFn { args: 1 }, span);

        c.asm.push(
            Inst::Replace {
                offset: binding_offset,
            },
            binding_span,
        );
    } else {
        // call the `next` function to get the next level of iteration, bind the
        // result to the loop variable in the loop.
        c.asm.push(
            Inst::Copy {
                offset: iter_offset,
            },
            ast.iter.span(),
        );

        c.asm.push_with_comment(
            Inst::CallInstance {
                hash: *Protocol::NEXT,
                args: 0,
            },
            span,
            "next",
        );
        c.asm.push(
            Inst::Replace {
                offset: binding_offset,
            },
            binding_span,
        );
    }

    // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
    c.asm.iter_next(binding_offset, end_label, binding_span);

    let body_span = ast.body.span();
    let guard = c.scopes.push_child(body_span)?;

    pat_with_offset(&ast.binding, c, binding_offset)?;

    block(&ast.body, c, Needs::None)?.apply(c)?;
    c.clean_last_scope(span, guard, Needs::None)?;

    c.asm.jump(continue_label, span);
    c.asm.label(end_label)?;

    // Drop the iterator.
    c.asm.push(
        Inst::Drop {
            offset: iter_offset,
        },
        span,
    );

    c.clean_last_scope(span, loop_scope_expected, Needs::None)?;

    // NB: If a value is needed from a for loop, encode it as a unit.
    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    // NB: breaks produce their own value.
    c.asm.label(break_label)?;
    Ok(Asm::top(span))
}

/// Assemble an if expression.
#[instrument]
fn expr_if(ast: &ast::ExprIf, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let then_label = c.asm.new_label("if_then");
    let end_label = c.asm.new_label("if_end");

    let mut branches = Vec::new();
    let then_scope = condition(&ast.condition, c, then_label)?;

    for branch in &ast.expr_else_ifs {
        let label = c.asm.new_label("if_branch");
        let scope = condition(&branch.condition, c, label)?;
        branches.push((branch, label, scope));
    }

    // use fallback as fall through.
    if let Some(fallback) = &ast.expr_else {
        block(&fallback.block, c, needs)?.apply(c)?;
    } else {
        // NB: if we must produce a value and there is no fallback branch,
        // encode the result of the statement as a unit.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }
    }

    c.asm.jump(end_label, span);

    c.asm.label(then_label)?;

    let expected = c.scopes.push(then_scope);
    block(&ast.block, c, needs)?.apply(c)?;
    c.clean_last_scope(span, expected, needs)?;

    if !ast.expr_else_ifs.is_empty() {
        c.asm.jump(end_label, span);
    }

    let mut it = branches.into_iter().peekable();
    while let Some((branch, label, scope)) = it.next() {
        let span = branch.span();

        c.asm.label(label)?;

        let scopes = c.scopes.push(scope);
        block(&branch.block, c, needs)?.apply(c)?;
        c.clean_last_scope(span, scopes, needs)?;

        if it.peek().is_some() {
            c.asm.jump(end_label, span);
        }
    }

    c.asm.label(end_label)?;
    Ok(Asm::top(span))
}

/// Assemble an expression.
#[instrument]
fn expr_index(ast: &ast::ExprIndex, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let guard = c.scopes.push_child(span)?;

    let target = expr(&ast.target, c, Needs::Value)?.apply_targeted(c)?;
    let index = expr(&ast.index, c, Needs::Value)?.apply_targeted(c)?;

    c.asm.push(Inst::IndexGet { index, target }, span);

    // NB: we still need to perform the operation since it might have side
    // effects, but pop the result in case a value is not needed.
    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    c.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a let expression.
#[instrument]
fn expr_let(ast: &ast::ExprLet, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let load = |c: &mut Assembler, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(&ast.expr, c, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(&ast.pat, c, false_label, &load)? {
        c.diagnostics
            .let_pattern_might_panic(c.source_id, span, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(ok_label, span);
        c.asm.label(false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            span,
        );

        c.asm.label(ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    Ok(Asm::top(span))
}

/// Compile a loop.
#[instrument]
fn expr_loop(ast: &ast::ExprLoop, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let continue_label = c.asm.new_label("loop_continue");
    let break_label = c.asm.new_label("loop_break");

    let var_count = c.scopes.total_var_count(span)?;

    let _guard = c.loops.push(Loop {
        label: ast.label.map(|(label, _)| label),
        continue_label,
        continue_var_count: var_count,
        break_label,
        break_var_count: var_count,
        needs,
        drop: None,
    });

    c.asm.label(continue_label)?;
    block(&ast.body, c, Needs::None)?.apply(c)?;
    c.asm.jump(continue_label, span);
    c.asm.label(break_label)?;

    Ok(Asm::top(span))
}

#[instrument]
fn expr_match(ast: &ast::ExprMatch, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let expected_scopes = c.scopes.push_child(span)?;

    expr(&ast.expr, c, Needs::Value)?.apply(c)?;
    // Offset of the expression.
    let offset = c.scopes.decl_anon(span)?;

    let end_label = c.asm.new_label("match_end");
    let mut branches = Vec::new();

    for (branch, _) in &ast.branches {
        let span = branch.span();

        let branch_label = c.asm.new_label("match_branch");
        let match_false = c.asm.new_label("match_false");

        let scope = c.scopes.child(span)?;
        let parent_guard = c.scopes.push(scope);

        let load = move |this: &mut Assembler, needs: Needs| {
            if needs.value() {
                this.asm.push(Inst::Copy { offset }, span);
            }

            Ok(())
        };

        pat(&branch.pat, c, match_false, &load)?;

        let scope = if let Some((_, condition)) = &branch.condition {
            let span = condition.span();

            let scope = c.scopes.child(span)?;
            let guard = c.scopes.push(scope);

            expr(condition, c, Needs::Value)?.apply(c)?;
            c.clean_last_scope(span, guard, Needs::Value)?;
            let scope = c.scopes.pop(parent_guard, span)?;

            c.asm
                .pop_and_jump_if_not(scope.local_var_count, match_false, span);

            c.asm.jump(branch_label, span);
            scope
        } else {
            c.scopes.pop(parent_guard, span)?
        };

        c.asm.jump(branch_label, span);
        c.asm.label(match_false)?;

        branches.push((branch_label, scope));
    }

    // what to do in case nothing matches and the pattern doesn't have any
    // default match branch.
    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    c.asm.jump(end_label, span);

    let mut it = ast.branches.iter().zip(&branches).peekable();

    while let Some(((branch, _), (label, scope))) = it.next() {
        let span = branch.span();

        c.asm.label(*label)?;

        let expected = c.scopes.push(scope.clone());
        expr(&branch.body, c, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;

        if it.peek().is_some() {
            c.asm.jump(end_label, span);
        }
    }

    c.asm.label(end_label)?;

    // pop the implicit scope where we store the anonymous match variable.
    c.clean_last_scope(span, expected_scopes, needs)?;
    Ok(Asm::top(span))
}

/// Compile a literal object.
#[instrument]
fn expr_object(ast: &ast::ExprObject, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();
    let guard = c.scopes.push_child(span)?;

    let mut keys = Vec::<Box<str>>::new();
    let mut check_keys = Vec::new();
    let mut keys_dup = HashMap::new();

    for (assign, _) in &ast.assignments {
        let span = assign.span();
        let key = assign.key.resolve(resolve_context!(c.q))?;
        keys.push(key.as_ref().into());
        check_keys.push((key.as_ref().into(), assign.key.span()));

        if let Some(existing) = keys_dup.insert(key.into_owned(), span) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::DuplicateObjectKey {
                    existing,
                    object: span,
                },
            ));
        }
    }

    for (assign, _) in &ast.assignments {
        let span = assign.span();

        if let Some((_, e)) = &assign.assign {
            expr(e, c, Needs::Value)?.apply(c)?;
        } else {
            let key = assign.key.resolve(resolve_context!(c.q))?;
            let var = c.scopes.get_var(c.q.visitor, &*key, c.source_id, span)?;
            let comment = format!("name `{}`", key);
            var.copy(c, span, comment);
        }

        c.scopes.decl_anon(span)?;
    }

    let slot = c.q.unit.new_static_object_keys_iter(span, &keys)?;

    match &ast.ident {
        ast::ObjectIdent::Named(path) => {
            let named = c.convert_path(path)?;
            named.assert_not_generic()?;

            let meta = c.lookup_meta(path.span(), &named.item)?;

            match &meta.kind {
                PrivMetaKind::Struct {
                    variant: PrivVariantMeta::Unit,
                    ..
                } => {
                    check_object_fields(&HashSet::new(), check_keys, span, &meta.item.item)?;

                    let hash = Hash::type_hash(&meta.item.item);
                    c.asm.push(Inst::UnitStruct { hash }, span);
                }
                PrivMetaKind::Struct {
                    variant: PrivVariantMeta::Struct(st),
                    ..
                } => {
                    check_object_fields(&st.fields, check_keys, span, &meta.item.item)?;

                    let hash = Hash::type_hash(&meta.item.item);
                    c.asm.push(Inst::Struct { hash, slot }, span);
                }
                PrivMetaKind::Variant {
                    variant: PrivVariantMeta::Struct(st),
                    ..
                } => {
                    check_object_fields(&st.fields, check_keys, span, &meta.item.item)?;

                    let hash = Hash::type_hash(&meta.item.item);
                    c.asm.push(Inst::StructVariant { hash, slot }, span);
                }
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedLitObject { meta: meta.info() },
                    ));
                }
            };
        }
        ast::ObjectIdent::Anonymous(..) => {
            c.asm.push(Inst::Object { slot }, span);
        }
    }

    // No need to encode an object since the value is not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    c.scopes.pop(guard, span)?;
    return Ok(Asm::top(span));

    fn check_object_fields(
        fields: &HashSet<Box<str>>,
        check_keys: Vec<(Box<str>, Span)>,
        span: Span,
        item: &Item,
    ) -> CompileResult<()> {
        let mut fields = fields.clone();

        for (field, span) in check_keys {
            if !fields.remove(&field) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::LitObjectNotField {
                        field,
                        item: item.clone(),
                    },
                ));
            }
        }

        if let Some(field) = fields.into_iter().next() {
            return Err(CompileError::new(
                span,
                CompileErrorKind::LitObjectMissingField {
                    field,
                    item: item.clone(),
                },
            ));
        }

        Ok(())
    }
}

/// Assemble a path.
#[instrument]
fn path(ast: &ast::Path, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    if let Some(ast::PathKind::SelfValue) = ast.as_kind() {
        let var = c.scopes.get_var(c.q.visitor, SELF, c.source_id, span)?;

        if needs.value() {
            var.copy(c, span, SELF);
        }

        return Ok(Asm::top(span));
    }

    let named = c.convert_path(ast)?;

    if let Needs::Value = needs {
        if let Some(local) = named.as_local() {
            if let Some(var) = c
                .scopes
                .try_get_var(c.q.visitor, local, c.source_id, span)?
            {
                return Ok(Asm::var(span, var, local.into()));
            }
        }
    }

    if let Some(m) = c.try_lookup_meta(span, &named.item)? {
        meta(span, c, &m, needs, named)?;
        return Ok(Asm::top(span));
    }

    if let (Needs::Value, Some(local)) = (needs, named.as_local()) {
        // light heuristics, treat it as a type error in case the
        // first character is uppercase.
        if !local.starts_with(char::is_uppercase) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::MissingLocal {
                    name: local.to_owned(),
                },
            ));
        }
    }

    Err(CompileError::new(
        span,
        CompileErrorKind::MissingItem {
            item: named.item.clone(),
        },
    ))
}

/// Assemble a range expression.
#[instrument]
fn expr_range(ast: &ast::ExprRange, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let guard = c.scopes.push_child(span)?;

    if needs.value() {
        let from = if let Some(from) = &ast.from {
            expr(from, c, needs)?.apply(c)?;
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::Some,
                },
                from.span(),
            );
            from.span()
        } else {
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::None,
                },
                span,
            );
            span
        };

        c.scopes.decl_anon(from)?;

        let to = if let Some(to) = &ast.to {
            expr(to, c, needs)?.apply(c)?;
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::Some,
                },
                to.span(),
            );
            to.span()
        } else {
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::None,
                },
                span,
            );
            span
        };

        c.scopes.decl_anon(to)?;

        let limits = match &ast.limits {
            ast::ExprRangeLimits::HalfOpen(..) => InstRangeLimits::HalfOpen,
            ast::ExprRangeLimits::Closed(..) => InstRangeLimits::Closed,
        };

        c.asm.push(Inst::Range { limits }, span);
        c.scopes.undecl_anon(span, 2)?;
    } else {
        if let Some(from) = &ast.from {
            expr(from, c, needs)?.apply(c)?;
        }

        if let Some(to) = &ast.to {
            expr(to, c, needs)?.apply(c)?;
        }
    }

    c.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a return expression.
#[instrument]
fn expr_return(ast: &ast::ExprReturn, c: &mut Assembler<'_>, _: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    // NB: drop any loop temporaries.
    for l in c.loops.iter() {
        if let Some(offset) = l.drop {
            c.asm.push(Inst::Drop { offset }, span);
        }
    }

    if let Some(e) = ast.expr.as_deref() {
        return_(c, span, e, expr)?;
    } else {
        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let clean = c.scopes.total_var_count(span)?;
        c.locals_pop(clean, span);
        c.asm.push(Inst::ReturnUnit, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a select expression.
#[instrument]
fn expr_select(ast: &ast::ExprSelect, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let len = ast.branches.len();
    c.contexts.push(span);

    let mut default_branch = None;
    let mut branches = Vec::new();

    let end_label = c.asm.new_label("select_end");

    for (branch, _) in &ast.branches {
        match branch {
            ast::ExprSelectBranch::Pat(pat) => {
                let label = c.asm.new_label("select_branch");
                branches.push((label, pat));
            }
            ast::ExprSelectBranch::Default(def) => {
                if default_branch.is_some() {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::SelectMultipleDefaults,
                    ));
                }

                let label = c.asm.new_label("select_default");
                default_branch = Some((def, label));
            }
        }
    }

    for (_, branch) in &branches {
        expr(&branch.expr, c, Needs::Value)?.apply(c)?;
    }

    c.asm.push(Inst::Select { len }, span);

    for (branch, (label, _)) in branches.iter().enumerate() {
        c.asm.jump_if_branch(branch as i64, *label, span);
    }

    if let Some((_, label)) = &default_branch {
        c.asm.push(Inst::Pop, span);
        c.asm.jump(*label, span);
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    c.asm.jump(end_label, span);

    for (label, branch) in branches {
        let span = branch.span();
        c.asm.label(label)?;

        let expected = c.scopes.push_child(span)?;

        // NB: loop is actually useful.
        #[allow(clippy::never_loop)]
        loop {
            match &branch.pat {
                ast::Pat::PatPath(path) => {
                    let named = c.convert_path(&path.path)?;
                    named.assert_not_generic()?;

                    if let Some(local) = named.as_local() {
                        c.scopes.decl_var(local, path.span())?;
                        break;
                    }
                }
                ast::Pat::PatIgnore(..) => {
                    c.asm.push(Inst::Pop, span);
                    break;
                }
                _ => (),
            }

            return Err(CompileError::new(
                branch,
                CompileErrorKind::UnsupportedSelectPattern,
            ));
        }

        // Set up a new scope with the binding.
        expr(&branch.body, c, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;
        c.asm.jump(end_label, span);
    }

    if let Some((branch, label)) = default_branch {
        c.asm.label(label)?;
        expr(&branch.body, c, needs)?.apply(c)?;
    }

    c.asm.label(end_label)?;

    c.contexts
        .pop()
        .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

    Ok(Asm::top(span))
}

/// Assemble a try expression.
#[instrument]
fn expr_try(ast: &ast::ExprTry, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let clean = c.scopes.total_var_count(span)?;
    let address = expr(&ast.expr, c, Needs::Value)?.apply_targeted(c)?;

    c.asm.push(
        Inst::Try {
            address,
            clean,
            preserve: needs.value(),
        },
        span,
    );

    if let InstAddress::Top = address {
        c.scopes.undecl_anon(span, 1)?;
    }

    // Why no needs.value() check here to declare another anonymous
    // variable? Because when these assembling functions were initially
    // implemented it was decided that the caller that indicates
    // Needs::Value is responsible for declaring any anonymous variables.
    //
    // TODO: This should probably change!

    Ok(Asm::top(span))
}

/// Assemble a literal tuple.
#[instrument]
fn expr_tuple(ast: &ast::ExprTuple, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    macro_rules! tuple {
        ($variant:ident, $c:ident, $span:expr, $($var:ident),*) => {{
            let guard = $c.scopes.push_child($span)?;

            let mut it = ast.items.iter();

            $(
            let ($var, _) = it.next().ok_or_else(|| CompileError::new($span, CompileErrorKind::Custom { message: "items ended unexpectedly" }))?;
            let $var = expr($var, $c, Needs::Value)?.apply_targeted($c)?;
            )*

            $c.asm.push(
                Inst::$variant {
                    args: [$($var,)*],
                },
                $span,
            );

            $c.scopes.pop(guard, $span)?;
        }};
    }

    let span = ast.span();

    if ast.items.is_empty() {
        c.asm.push(Inst::unit(), span);
    } else {
        match ast.items.len() {
            1 => tuple!(Tuple1, c, span, e1),
            2 => tuple!(Tuple2, c, span, e1, e2),
            3 => tuple!(Tuple3, c, span, e1, e2, e3),
            4 => tuple!(Tuple4, c, span, e1, e2, e3, e4),
            _ => {
                for (e, _) in &ast.items {
                    expr(e, c, Needs::Value)?.apply(c)?;
                    c.scopes.decl_anon(e.span())?;
                }

                c.asm.push(
                    Inst::Tuple {
                        count: ast.items.len(),
                    },
                    span,
                );

                c.scopes.undecl_anon(span, ast.items.len())?;
            }
        }
    }

    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a unary expression.
#[instrument]
fn expr_unary(ast: &ast::ExprUnary, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    // NB: special unary expressions.
    if let ast::UnOp::BorrowRef { .. } = ast.op {
        return Err(CompileError::new(ast, CompileErrorKind::UnsupportedRef));
    }

    if let (ast::UnOp::Neg(..), ast::Expr::Lit(expr_lit)) = (ast.op, &*ast.expr) {
        if let ast::Lit::Number(n) = &expr_lit.lit {
            match n.resolve(resolve_context!(c.q))? {
                ast::Number::Float(n) => {
                    c.asm.push(Inst::float(-n), span);
                }
                ast::Number::Integer(int) => {
                    use num::ToPrimitive as _;
                    use std::ops::Neg as _;

                    let n = match int.neg().to_i64() {
                        Some(n) => n,
                        None => {
                            return Err(CompileError::new(
                                span,
                                ParseErrorKind::BadNumberOutOfBounds,
                            ));
                        }
                    };

                    c.asm.push(Inst::integer(n), span);
                }
            }

            return Ok(Asm::top(span));
        }
    }

    expr(&ast.expr, c, Needs::Value)?.apply(c)?;

    match ast.op {
        ast::UnOp::Not(..) => {
            c.asm.push(Inst::Not, span);
        }
        ast::UnOp::Neg(..) => {
            c.asm.push(Inst::Neg, span);
        }
        op => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::UnsupportedUnaryOp { op },
            ));
        }
    }

    // NB: we put it here to preserve the call in case it has side effects.
    // But if we don't need the value, then pop it from the stack.
    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a literal vector.
#[instrument]
fn expr_vec(ast: &ast::ExprVec, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let count = ast.items.len();

    for (e, _) in &ast.items {
        expr(e, c, Needs::Value)?.apply(c)?;
        c.scopes.decl_anon(e.span())?;
    }

    c.asm.push(Inst::Vec { count }, span);
    c.scopes.undecl_anon(span, ast.items.len())?;

    // Evaluate the expressions one by one, then pop them to cause any
    // side effects (without creating an object).
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a while loop.
#[instrument]
fn expr_while(ast: &ast::ExprWhile, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let continue_label = c.asm.new_label("while_continue");
    let then_label = c.asm.new_label("whiel_then");
    let end_label = c.asm.new_label("while_end");
    let break_label = c.asm.new_label("while_break");

    let var_count = c.scopes.total_var_count(span)?;

    let _guard = c.loops.push(Loop {
        label: ast.label.map(|(label, _)| label),
        continue_label,
        continue_var_count: var_count,
        break_label,
        break_var_count: var_count,
        needs,
        drop: None,
    });

    c.asm.label(continue_label)?;

    let then_scope = condition(&ast.condition, c, then_label)?;
    let expected = c.scopes.push(then_scope);

    c.asm.jump(end_label, span);
    c.asm.label(then_label)?;

    block(&ast.body, c, Needs::None)?.apply(c)?;
    c.clean_last_scope(span, expected, Needs::None)?;

    c.asm.jump(continue_label, span);
    c.asm.label(end_label)?;

    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    // NB: breaks produce their own value / perform their own cleanup.
    c.asm.label(break_label)?;
    Ok(Asm::top(span))
}

/// Assemble a `yield` expression.
#[instrument]
fn expr_yield(ast: &ast::ExprYield, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    if let Some(e) = &ast.expr {
        expr(e, c, Needs::Value)?.apply(c)?;
        c.asm.push(Inst::Yield, span);
    } else {
        c.asm.push(Inst::YieldUnit, span);
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a function from an [ast::ItemFn].
#[instrument]
pub(crate) fn fn_from_item_fn(
    ast: &ast::ItemFn,
    c: &mut Assembler<'_>,
    instance_fn: bool,
) -> CompileResult<()> {
    let span = ast.span();

    let mut patterns = Vec::new();
    let mut first = true;

    for (arg, _) in &ast.args {
        let span = arg.span();

        match arg {
            ast::FnArg::SelfValue(s) => {
                if !instance_fn || !first {
                    return Err(CompileError::new(span, CompileErrorKind::UnsupportedSelf));
                }

                let span = s.span();
                c.scopes.new_var(SELF, span)?;
            }
            ast::FnArg::Pat(pat) => {
                let offset = c.scopes.decl_anon(pat.span())?;
                patterns.push((pat, offset));
            }
        }

        first = false;
    }

    for (pat, offset) in patterns {
        pat_with_offset(pat, c, offset)?;
    }

    if ast.body.statements.is_empty() {
        let total_var_count = c.scopes.total_var_count(span)?;
        c.locals_pop(total_var_count, span);
        c.asm.push(Inst::ReturnUnit, span);
        return Ok(());
    }

    if !ast.body.produces_nothing() {
        return_(c, span, &ast.body, block)?;
    } else {
        block(&ast.body, c, Needs::None)?.apply(c)?;

        let total_var_count = c.scopes.total_var_count(span)?;
        c.locals_pop(total_var_count, span);
        c.asm.push(Inst::ReturnUnit, span);
    }

    c.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a literal value.
#[instrument]
fn lit(ast: &ast::Lit, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    match ast {
        ast::Lit::Bool(lit) => {
            c.asm.push(Inst::bool(lit.value), span);
        }
        ast::Lit::Number(lit) => {
            return lit_number(lit, c, needs);
        }
        ast::Lit::Char(lit) => {
            let ch = lit.resolve(resolve_context!(c.q))?;
            c.asm.push(Inst::char(ch), span);
        }
        ast::Lit::Str(lit) => {
            return lit_str(lit, c, needs);
        }
        ast::Lit::Byte(lit) => {
            let b = lit.resolve(resolve_context!(c.q))?;
            c.asm.push(Inst::byte(b), span);
        }
        ast::Lit::ByteStr(lit) => {
            let bytes = lit.resolve(resolve_context!(c.q))?;
            let slot = c.q.unit.new_static_bytes(span, &*bytes)?;
            c.asm.push(Inst::Bytes { slot }, span);
        }
    };

    Ok(Asm::top(span))
}

#[instrument]
fn lit_str(ast: &ast::LitStr, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    let string = ast.resolve(resolve_context!(c.q))?;
    let slot = c.q.unit.new_static_string(span, &*string)?;
    c.asm.push(Inst::String { slot }, span);
    Ok(Asm::top(span))
}

/// Assemble a literal number.
#[instrument]
fn lit_number(ast: &ast::LitNumber, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    use num::ToPrimitive;

    let span = ast.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    // NB: don't encode unecessary literal.
    let number = ast.resolve(resolve_context!(c.q))?;

    match number {
        ast::Number::Float(number) => {
            c.asm.push(Inst::float(number), span);
        }
        ast::Number::Integer(number) => {
            let n = match number.to_i64() {
                Some(n) => n,
                None => {
                    return Err(CompileError::new(
                        span,
                        ParseErrorKind::BadNumberOutOfBounds,
                    ));
                }
            };

            c.asm.push(Inst::integer(n), span);
        }
    }

    Ok(Asm::top(span))
}

/// Assemble a local expression.
#[instrument]
fn local(ast: &ast::Local, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = ast.span();

    let load = |c: &mut Assembler, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(&ast.expr, c, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(&ast.pat, c, false_label, &load)? {
        c.diagnostics
            .let_pattern_might_panic(c.source_id, span, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(ok_label, span);
        c.asm.label(false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            span,
        );

        c.asm.label(ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    Ok(Asm::top(span))
}

/// Test if the given pattern is open or not.
fn pat_items_count<'a, I: 'a, U: 'a>(items: I) -> Result<(bool, usize), CompileError>
where
    I: IntoIterator<Item = &'a (ast::Pat, U)>,
    I::IntoIter: DoubleEndedIterator,
{
    let mut it = items.into_iter();

    let (is_open, mut count) = match it.next_back() {
        Some((pat, _)) => {
            if matches!(pat, ast::Pat::PatRest(..)) {
                (true, 0)
            } else {
                (false, 1)
            }
        }
        None => return Ok((false, 0)),
    };

    for (pat, _) in it {
        if let ast::Pat::PatRest(rest) = pat {
            return Err(CompileError::new(
                rest,
                CompileErrorKind::UnsupportedPatternRest,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}

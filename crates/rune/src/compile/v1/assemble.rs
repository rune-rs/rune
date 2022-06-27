use std::convert::TryFrom;
use std::ops::Neg;

use num::ToPrimitive;
use rune_macros::__instrument_ast as instrument;

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::collections::{HashMap, HashSet};
use crate::compile::v1::{Assembler, Loop, Needs, Scope, Var};
use crate::compile::{
    CaptureMeta, CompileError, CompileErrorKind, CompileResult, Item, PrivMeta, PrivMetaKind,
    PrivStructMeta, PrivVariantMeta,
};
use crate::hash::ParametersBuilder;
use crate::hir;
use crate::parse::{Id, ParseErrorKind, Resolve};
use crate::query::Named;
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget, InstValue,
    InstVariant, Label, PanicReason, Protocol, TypeCheck,
};
use crate::Hash;

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
                let hash = if let Some((span, generics)) = named.generics {
                    let parameters = generics_parameters(span, c, generics)?;
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
    hir: &T,
    asm: impl FnOnce(&T, &mut Assembler<'_>, Needs) -> CompileResult<Asm>,
) -> CompileResult<()> {
    let clean = c.scopes.total_var_count(span)?;

    let address = asm(hir, c, Needs::Value)?.apply_targeted(c)?;
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
fn pat_with_offset(hir: &hir::Pat<'_>, c: &mut Assembler<'_>, offset: usize) -> CompileResult<()> {
    let span = hir.span();

    let load = |c: &mut Assembler, needs: Needs| {
        if needs.value() {
            c.asm.push(Inst::Copy { offset }, span);
        }

        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(hir, c, false_label, &load)? {
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
    hir: &hir::Pat<'_>,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<bool> {
    let span = hir.span();

    match hir.kind {
        hir::PatKind::PatIgnore => {
            // ignore binding, but might still have side effects, so must
            // call the load generator.
            load(c, Needs::None)?;
            Ok(false)
        }
        hir::PatKind::PatPath(path) => {
            let named = c.convert_path(path)?;
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
        hir::PatKind::PatLit(hir) => Ok(pat_lit(hir, c, false_label, load)?),
        hir::PatKind::PatVec(hir) => {
            pat_vec(span, c, hir, false_label, &load)?;
            Ok(true)
        }
        hir::PatKind::PatTuple(hir) => {
            pat_tuple(span, c, hir, false_label, &load)?;
            Ok(true)
        }
        hir::PatKind::PatObject(hir) => {
            pat_object(span, c, hir, false_label, &load)?;
            Ok(true)
        }
        _ => Err(CompileError::new(
            hir,
            CompileErrorKind::UnsupportedPatternExpr,
        )),
    }
}

/// Assemble a pattern literal.
#[instrument]
fn pat_lit(
    hir: &hir::Expr<'_>,
    c: &mut Assembler<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<bool> {
    let span = hir.span();

    let inst = match pat_lit_inst(span, c, hir)? {
        Some(inst) => inst,
        None => {
            return Err(CompileError::new(
                hir,
                CompileErrorKind::UnsupportedPatternExpr,
            ));
        }
    };

    load(c, Needs::Value)?;
    c.asm.push(inst, span);
    c.asm
        .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);
    Ok(true)
}

#[instrument]
fn pat_lit_inst(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::Expr<'_>,
) -> CompileResult<Option<Inst>> {
    match hir.kind {
        hir::ExprKind::Unary(hir::ExprUnary {
            op: ast::UnOp::Neg(..),
            expr:
                hir::Expr {
                    kind: hir::ExprKind::Lit(ast::Lit::Number(lit)),
                    ..
                },
            ..
        }) => {
            let integer = lit.resolve(resolve_context!(c.q))?.as_i64(span, true)?;
            return Ok(Some(Inst::EqInteger { integer }));
        }
        hir::ExprKind::Lit(lit) => match lit {
            ast::Lit::Byte(lit) => {
                let byte = lit.resolve(resolve_context!(c.q))?;
                return Ok(Some(Inst::EqByte { byte }));
            }
            ast::Lit::Char(lit) => {
                let char = lit.resolve(resolve_context!(c.q))?;
                return Ok(Some(Inst::EqChar { char }));
            }
            ast::Lit::Str(lit) => {
                let string = lit.resolve(resolve_context!(c.q))?;
                let slot = c.q.unit.new_static_string(span, string.as_ref())?;
                return Ok(Some(Inst::EqString { slot }));
            }
            ast::Lit::ByteStr(lit) => {
                let bytes = lit.resolve(resolve_context!(c.q))?;
                let slot = c.q.unit.new_static_bytes(span, bytes.as_ref())?;
                return Ok(Some(Inst::EqBytes { slot }));
            }
            ast::Lit::Number(lit) => {
                let integer = lit.resolve(resolve_context!(c.q))?.as_i64(span, false)?;
                return Ok(Some(Inst::EqInteger { integer }));
            }
            ast::Lit::Bool(lit) => {
                return Ok(Some(Inst::EqBool { boolean: lit.value }));
            }
        },
        _ => (),
    }

    Ok(None)
}

/// Assemble an [hir::Condition<'_>].
#[instrument]
fn condition(
    condition: &hir::Condition<'_>,
    c: &mut Assembler<'_>,
    then_label: Label,
) -> CompileResult<Scope> {
    match condition {
        hir::Condition::Expr(e) => {
            let span = e.span();

            expr(e, c, Needs::Value)?.apply(c)?;
            c.asm.jump_if(then_label, span);

            Ok(c.scopes.child(span)?)
        }
        hir::Condition::ExprLet(expr_let) => {
            let span = expr_let.span();

            let false_label = c.asm.new_label("if_condition_false");

            let scope = c.scopes.child(span)?;
            let expected = c.scopes.push(scope);

            let load = |c: &mut Assembler<'_>, needs: Needs| {
                expr(expr_let.expr, c, needs)?.apply(c)?;
                Ok(())
            };

            if pat(expr_let.pat, c, false_label, &load)? {
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
    span: Span,
    c: &mut Assembler<'_>,
    hir: &[hir::Pat<'_>],
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    load(c, Needs::Value)?;
    let offset = c.scopes.decl_anon(span)?;

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    c.asm.push(Inst::Copy { offset }, span);

    let (is_open, count) = pat_items_count(hir)?;

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

    for (index, hir) in hir.iter().take(count).enumerate() {
        let load = move |c: &mut Assembler<'_>, needs: Needs| {
            if needs.value() {
                c.asm
                    .push(Inst::TupleIndexGetAt { offset, index }, hir.span());
            }

            Ok(())
        };

        pat(hir, c, false_label, &load)?;
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
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::PatItems<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    load(c, Needs::Value)?;

    if hir.items.is_empty() {
        c.asm.push(Inst::IsUnit, span);

        c.asm
            .pop_and_jump_if_not(c.scopes.local_var_count(span)?, false_label, span);
        return Ok(());
    }

    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    let offset = c.scopes.decl_anon(span)?;

    let (is_open, count) = pat_items_count(hir.items)?;

    if let Some(path) = hir.path {
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

    for (index, p) in hir.items.iter().take(count).enumerate() {
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
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::PatItems<'_>,
    false_label: Label,
    load: &dyn Fn(&mut Assembler<'_>, Needs) -> CompileResult<()>,
) -> CompileResult<()> {
    // NB: bind the loaded variable (once) to an anonymous var.
    // We reduce the number of copy operations by having specialized
    // operations perform the load from the given offset.
    load(c, Needs::Value)?;
    let offset = c.scopes.decl_anon(span)?;

    let mut string_slots = Vec::new();
    let mut keys_dup = HashMap::new();
    let mut keys = Vec::new();
    let mut bindings = Vec::new();

    let (has_rest, count) = pat_items_count(hir.items)?;

    for pat in hir.items.iter().take(count) {
        let span = pat.span();
        let cow_key;

        let key = match pat.kind {
            hir::PatKind::PatBinding(binding) => {
                cow_key = binding.key.resolve(resolve_context!(c.q))?;
                bindings.push(Binding::Binding(
                    pat.span(),
                    cow_key.as_ref().into(),
                    binding.pat,
                ));
                cow_key.as_ref()
            }
            hir::PatKind::PatPath(path) => {
                let ident = match path.try_as_ident() {
                    Some(ident) => ident,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedPatternExpr,
                        ));
                    }
                };

                let key = ident.resolve(resolve_context!(c.q))?;
                bindings.push(Binding::Ident(pat.span(), key.into()));
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
                    object: span,
                },
            ));
        }

        keys.push(key.to_string());
    }

    match hir.path {
        Some(path) => {
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
                    span,
                    CompileErrorKind::PatternMissingFields {
                        item: meta.item.item.clone(),
                        fields,
                    },
                ));
            }

            c.asm.push(Inst::Copy { offset }, span);
            c.asm.push(inst, span);
        }
        None => {
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

    enum Binding<'hir> {
        Binding(Span, Box<str>, &'hir hir::Pat<'hir>),
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
    hir: &hir::Block<'_>,
    c: &mut Assembler<'_>,
    captures: &[CaptureMeta],
) -> CompileResult<()> {
    let span = hir.span();

    let guard = c.scopes.push_child(span)?;

    for capture in captures {
        c.scopes.new_var(&capture.ident, span)?;
    }

    return_(c, span, hir, block)?;
    c.scopes.pop(guard, span)?;
    Ok(())
}

/// Call a block.
#[instrument]
fn block(hir: &hir::Block<'_>, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    c.contexts.push(span);
    let scopes_count = c.scopes.push_child(span)?;

    let mut last = None::<(&hir::Expr<'_>, bool)>;

    for stmt in hir.statements {
        let (e, semi) = match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = std::mem::take(&mut last) {
                    // NB: terminated expressions do not need to produce a value.
                    expr(e, c, Needs::None)?.apply(c)?;
                }

                local(l, c, Needs::None)?.apply(c)?;
                continue;
            }
            hir::Stmt::Expr(expr) => (expr, false),
            hir::Stmt::Semi(expr) => (expr, true),
            hir::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = std::mem::replace(&mut last, Some((e, semi))) {
            // NB: terminated expressions do not need to produce a value.
            expr(e, c, Needs::None)?.apply(c)?;
        }
    }

    let produced = if let Some((e, semi)) = last {
        if semi {
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
fn builtin_format(
    format: &hir::BuiltInFormat<'_>,
    c: &mut Assembler<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    use crate::runtime::format;

    let span = format.span();

    let fill = if let Some((_, fill)) = &format.fill {
        *fill
    } else {
        ' '
    };

    let align = if let Some((_, align)) = &format.align {
        *align
    } else {
        format::Alignment::default()
    };

    let flags = if let Some((_, flags)) = &format.flags {
        *flags
    } else {
        format::Flags::default()
    };

    let width = if let Some((_, width)) = &format.width {
        *width
    } else {
        None
    };

    let precision = if let Some((_, precision)) = &format.precision {
        *precision
    } else {
        None
    };

    let format_type = if let Some((_, format_type)) = &format.format_type {
        *format_type
    } else {
        format::Type::default()
    };

    let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

    expr(format.value, c, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Format { spec }, span);

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument]
fn builtin_template(
    template: &hir::BuiltInTemplate<'_>,
    c: &mut Assembler<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let span = template.span();

    let expected = c.scopes.push_child(span)?;
    let mut size_hint = 0;
    let mut expansions = 0;

    for hir in template.exprs {
        if let hir::ExprKind::Lit(ast::Lit::Str(s)) = hir.kind {
            let s = s.resolve_template_string(resolve_context!(c.q))?;
            size_hint += s.len();

            let slot = c.q.unit.new_static_string(span, &s)?;
            c.asm.push(Inst::String { slot }, span);
            c.scopes.decl_anon(span)?;
            continue;
        }

        expansions += 1;

        expr(hir, c, Needs::Value)?.apply(c)?;
        c.scopes.decl_anon(span)?;
    }

    if template.from_literal && expansions == 0 {
        c.diagnostics
            .template_without_expansions(c.source_id, span, c.context());
    }

    c.asm.push(
        Inst::StringConcat {
            len: template.exprs.len(),
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
            let slot = c.q.unit.new_static_bytes(span, b)?;
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
fn expr(hir: &hir::Expr<'_>, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    let asm = match hir.kind {
        hir::ExprKind::Path(p) => path(p, c, needs)?,
        hir::ExprKind::While(hir) => expr_while(span, c, hir, needs)?,
        hir::ExprKind::For(hir) => expr_for(span, c, hir, needs)?,
        hir::ExprKind::Loop(hir) => expr_loop(span, c, hir, needs)?,
        hir::ExprKind::Let(hir) => expr_let(hir, c, needs)?,
        hir::ExprKind::Group(hir) => expr(hir, c, needs)?,
        hir::ExprKind::Unary(hir) => expr_unary(span, c, hir, needs)?,
        hir::ExprKind::Assign(hir) => expr_assign(span, c, hir, needs)?,
        hir::ExprKind::Binary(hir) => expr_binary(span, c, hir, needs)?,
        hir::ExprKind::If(hir) => expr_if(span, c, hir, needs)?,
        hir::ExprKind::Index(hir) => expr_index(span, c, hir, needs)?,
        hir::ExprKind::Break(hir) => expr_break(span, c, hir, needs)?,
        hir::ExprKind::Continue(hir) => expr_continue(span, c, hir, needs)?,
        hir::ExprKind::Yield(hir) => expr_yield(span, c, hir, needs)?,
        hir::ExprKind::Block(hir) => expr_block(span, c, hir, needs)?,
        hir::ExprKind::Return(hir) => expr_return(span, c, hir, needs)?,
        hir::ExprKind::Match(hir) => expr_match(span, c, hir, needs)?,
        hir::ExprKind::Await(hir) => expr_await(span, c, hir, needs)?,
        hir::ExprKind::Try(hir) => expr_try(span, c, hir, needs)?,
        hir::ExprKind::Select(hir) => expr_select(span, c, hir, needs)?,
        hir::ExprKind::Call(hir) => expr_call(span, c, hir, needs)?,
        hir::ExprKind::FieldAccess(hir) => expr_field_access(span, c, hir, needs)?,
        hir::ExprKind::Closure(hir) => expr_closure(span, c, hir, needs)?,
        hir::ExprKind::Lit(hir) => lit(hir, c, needs)?,
        hir::ExprKind::Tuple(hir) => expr_tuple(span, c, hir, needs)?,
        hir::ExprKind::Vec(hir) => expr_vec(span, c, hir, needs)?,
        hir::ExprKind::Object(hir) => expr_object(span, c, hir, needs)?,
        hir::ExprKind::Range(hir) => expr_range(span, c, hir, needs)?,
        hir::ExprKind::MacroCall(macro_call) => match macro_call {
            hir::MacroCall::Template(template) => builtin_template(template, c, needs)?,
            hir::MacroCall::Format(format) => builtin_format(format, c, needs)?,
            hir::MacroCall::Line(line) => lit_number(&line.value, c, needs)?,
            hir::MacroCall::File(file) => lit_str(&file.value, c, needs)?,
        },
    };

    Ok(asm)
}

/// Assemble an assign expression.
#[instrument]
fn expr_assign(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprAssign<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Path(path) if path.rest.is_empty() => {
            expr(hir.rhs, c, Needs::Value)?.apply(c)?;

            let segment = path
                .first
                .try_as_ident()
                .ok_or_else(|| CompileError::msg(path, "unsupported path"))?;
            let ident = segment.resolve(resolve_context!(c.q))?;
            let var = c.scopes.get_var(c.q.visitor, ident, c.source_id, span)?;
            c.asm.push(Inst::Replace { offset: var.offset }, span);
            true
        }
        // <expr>.<field> = <value>
        hir::ExprKind::FieldAccess(field_access) => {
            // field assignment
            match field_access.expr_field {
                hir::ExprField::Path(path) => {
                    if let Some(ident) = path.try_as_ident() {
                        let slot = ident.resolve(resolve_context!(c.q))?;
                        let slot = c.q.unit.new_static_string(ident.span(), slot.as_ref())?;

                        expr(hir.rhs, c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(hir.rhs.span())?;

                        expr(field_access.expr, c, Needs::Value)?.apply(c)?;
                        c.scopes.decl_anon(span)?;

                        c.asm.push(Inst::ObjectIndexSet { slot }, span);
                        c.scopes.undecl_anon(span, 2)?;
                        true
                    } else {
                        false
                    }
                }
                hir::ExprField::LitNumber(field) => {
                    let number = field.resolve(resolve_context!(c.q))?;
                    let index = number.as_tuple_index().ok_or_else(|| {
                        CompileError::new(span, CompileErrorKind::UnsupportedTupleIndex { number })
                    })?;

                    expr(hir.rhs, c, Needs::Value)?.apply(c)?;
                    c.scopes.decl_anon(hir.rhs.span())?;

                    expr(field_access.expr, c, Needs::Value)?.apply(c)?;
                    c.asm.push(Inst::TupleIndexSet { index }, span);
                    c.scopes.undecl_anon(span, 1)?;
                    true
                }
            }
        }
        hir::ExprKind::Index(expr_index_get) => {
            expr(hir.rhs, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            expr(expr_index_get.target, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            expr(expr_index_get.index, c, Needs::Value)?.apply(c)?;
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
fn expr_await(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::Expr<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    expr(hir, c, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Await, span);

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a binary expression.
#[instrument]
fn expr_binary(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprBinary<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        compile_assign_binop(span, c, hir.lhs, hir.rhs, &hir.op, needs)?;
        return Ok(Asm::top(span));
    }

    if hir.op.is_conditional() {
        compile_conditional_binop(span, c, hir.lhs, hir.rhs, &hir.op, needs)?;
        return Ok(Asm::top(span));
    }

    let guard = c.scopes.push_child(span)?;

    // NB: need to declare these as anonymous local variables so that they
    // get cleaned up in case there is an early break (return, try, ...).
    let rhs_needs = rhs_needs_of(&hir.op);
    let a = expr(hir.lhs, c, Needs::Value)?.apply_targeted(c)?;
    let b = expr(hir.rhs, c, rhs_needs)?.apply_targeted(c)?;

    let op = match hir.op {
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
        span: Span,
        c: &mut Assembler<'_>,
        lhs: &hir::Expr<'_>,
        rhs: &hir::Expr<'_>,
        bin_op: &ast::BinOp,
        needs: Needs,
    ) -> CompileResult<()> {
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
        span: Span,
        c: &mut Assembler<'_>,
        lhs: &hir::Expr<'_>,
        rhs: &hir::Expr<'_>,
        bin_op: &ast::BinOp,
        needs: Needs,
    ) -> CompileResult<()> {
        let supported = match lhs.kind {
            // <var> <op> <expr>
            hir::ExprKind::Path(path) if path.rest.is_empty() => {
                expr(rhs, c, Needs::Value)?.apply(c)?;

                let segment = path
                    .first
                    .try_as_ident()
                    .ok_or_else(|| CompileError::msg(path, "unsupported path segment"))?;

                let ident = segment.resolve(resolve_context!(c.q))?;
                let var = c.scopes.get_var(c.q.visitor, ident, c.source_id, span)?;

                Some(InstTarget::Offset(var.offset))
            }
            // <expr>.<field> <op> <value>
            hir::ExprKind::FieldAccess(field_access) => {
                expr(field_access.expr, c, Needs::Value)?.apply(c)?;
                expr(rhs, c, Needs::Value)?.apply(c)?;

                // field assignment
                match field_access.expr_field {
                    hir::ExprField::Path(path) => {
                        if let Some(ident) = path.try_as_ident() {
                            let n = ident.resolve(resolve_context!(c.q))?;
                            let n = c.q.unit.new_static_string(path.span(), n.as_ref())?;

                            Some(InstTarget::Field(n))
                        } else {
                            None
                        }
                    }
                    hir::ExprField::LitNumber(field) => {
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
fn expr_block(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprBlock<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    if let hir::ExprBlockKind::Default = hir.kind {
        return block(hir.block, c, needs);
    }

    let item = c.q.item_for(hir.block)?;
    let meta = c.lookup_meta(span, &item.item)?;

    match (hir.kind, &meta.kind) {
        (
            hir::ExprBlockKind::Async,
            PrivMetaKind::AsyncBlock {
                captures, do_move, ..
            },
        ) => {
            let captures = captures.as_ref();
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
        (hir::ExprBlockKind::Const, PrivMetaKind::Const { const_value }) => {
            const_(span, c, const_value, needs)?;
        }
        _ => {
            return Err(CompileError::expected_meta(
                span,
                meta.info(),
                "async or const block",
            ));
        }
    };

    Ok(Asm::top(span))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument]
fn expr_break(
    span: Span,
    c: &mut Assembler<'_>,
    hir: Option<&hir::ExprBreakValue<'_>>,
    _: Needs,
) -> CompileResult<Asm> {
    let current_loop = match c.loops.last() {
        Some(current_loop) => current_loop,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::BreakOutsideOfLoop,
            ));
        }
    };

    let (last_loop, to_drop, has_value) = if let Some(e) = hir {
        match e {
            hir::ExprBreakValue::Expr(e) => {
                expr(e, c, current_loop.needs)?.apply(c)?;
                (current_loop, current_loop.drop.into_iter().collect(), true)
            }
            hir::ExprBreakValue::Label(label) => {
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

#[instrument]
fn generics_parameters(
    span: Span,
    c: &mut Assembler<'_>,
    generics: &[hir::Expr<'_>],
) -> Result<Hash, CompileError> {
    let mut parameters = ParametersBuilder::new();

    for expr in generics {
        let path = match expr.kind {
            hir::ExprKind::Path(path) => path,
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedGenerics,
                ));
            }
        };

        let named = c.convert_path(path)?;
        named.assert_not_generic()?;

        let meta = c.lookup_meta(expr.span(), &named.item)?;

        let hash = match meta.kind {
            PrivMetaKind::Unknown { type_hash, .. } => type_hash,
            PrivMetaKind::Struct { type_hash, .. } => type_hash,
            PrivMetaKind::Enum { type_hash, .. } => type_hash,
            _ => {
                return Err(CompileError::new(
                    span,
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
#[instrument]
fn convert_expr_call(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprCall<'_>,
) -> CompileResult<Call> {
    match hir.expr.kind {
        hir::ExprKind::Path(path) => {
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

                    if !hir.args.is_empty() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedArgumentCount {
                                meta: meta.info(),
                                expected: 0,
                                actual: hir.args.len(),
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

                    if tuple.args != hir.args.len() {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedArgumentCount {
                                meta: meta.info(),
                                expected: tuple.args,
                                actual: hir.args.len(),
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

            let hash = if let Some((span, generics)) = named.generics {
                let parameters = generics_parameters(span, c, generics)?;
                hash.with_parameters(parameters)
            } else {
                hash
            };

            return Ok(Call::Meta { meta, hash });
        }
        hir::ExprKind::FieldAccess(hir::ExprFieldAccess {
            expr_field: hir::ExprField::Path(path),
            ..
        }) => {
            if let Some((ident, generics)) = path.try_as_ident_generics() {
                let ident = ident.resolve(resolve_context!(c.q))?;
                let hash = Hash::instance_fn_name(ident);

                let hash = if let Some((span, generics)) = generics {
                    let parameters = generics_parameters(span, c, generics)?;
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
fn expr_call(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprCall<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let call = convert_expr_call(span, c, hir)?;

    let args = hir.args.len();

    match call {
        Call::Var { var, name } => {
            for e in hir.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            var.copy(c, span, format!("var `{}`", name));
            c.scopes.decl_anon(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.undecl_anon(span, hir.args.len() + 1)?;
        }
        Call::Instance { hash } => {
            let target = hir.target();

            expr(target, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(target.span())?;

            for e in hir.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            c.asm.push(Inst::CallInstance { hash, args }, span);
            c.scopes.undecl_anon(span, hir.args.len() + 1)?;
        }
        Call::Meta { meta, hash } => {
            for e in hir.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            c.asm
                .push_with_comment(Inst::Call { hash, args }, span, meta.info().to_string());

            c.scopes.undecl_anon(span, args)?;
        }
        Call::Expr => {
            for e in hir.args {
                expr(e, c, Needs::Value)?.apply(c)?;
                c.scopes.decl_anon(span)?;
            }

            expr(hir.expr, c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.undecl_anon(span, args + 1)?;
        }
        Call::ConstFn { meta, id } => {
            let from = c.q.item_for((span, hir.id))?;
            let const_fn = c.q.const_fn_for((span, id))?;
            let value = c.call_const_fn(span, &meta, &from, &const_fn, hir.args)?;
            const_(span, c, &value, Needs::Value)?;
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
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprClosure<'_>,
    captures: &[CaptureMeta],
) -> CompileResult<()> {
    let mut patterns = Vec::new();

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(s) => {
                return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
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

    return_(c, span, hir.body, expr)?;
    c.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a closure expression.
#[instrument]
fn expr_closure(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprClosure<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    let item = c.q.item_for((span, hir.id))?;
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
        } => (captures.as_ref(), *do_move),
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
fn expr_continue(
    span: Span,
    c: &mut Assembler<'_>,
    hir: Option<&ast::Label>,
    _: Needs,
) -> CompileResult<Asm> {
    let current_loop = match c.loops.last() {
        Some(current_loop) => current_loop,
        None => {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ContinueOutsideOfLoop,
            ));
        }
    };

    let last_loop = if let Some(label) = hir {
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
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprFieldAccess<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    // Optimizations!
    //
    // TODO: perform deferred compilation for expressions instead, so we can
    // e.g. inspect if it compiles down to a local access instead of
    // climbing the hir like we do here.
    #[allow(clippy::single_match)]
    match (hir.expr.kind, hir.expr_field) {
        (hir::ExprKind::Path(path), hir::ExprField::LitNumber(n)) => {
            if try_immediate_field_access_optimization(c, span, path, n, needs)? {
                return Ok(Asm::top(span));
            }
        }
        _ => (),
    }

    expr(hir.expr, c, Needs::Value)?.apply(c)?;

    match hir.expr_field {
        hir::ExprField::LitNumber(n) => {
            if let Some(index) = n.resolve(resolve_context!(c.q))?.as_tuple_index() {
                c.asm.push(Inst::TupleIndexGet { index }, span);

                if !needs.value() {
                    c.diagnostics.not_used(c.source_id, span, c.context());
                    c.asm.push(Inst::Pop, span);
                }

                return Ok(Asm::top(span));
            }
        }
        hir::ExprField::Path(path) => {
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
        path: &hir::Path<'_>,
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
fn expr_for(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprFor<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let continue_label = c.asm.new_label("for_continue");
    let end_label = c.asm.new_label("for_end");
    let break_label = c.asm.new_label("for_break");

    let break_var_count = c.scopes.total_var_count(span)?;

    let (iter_offset, loop_scope_expected) = {
        let loop_scope_expected = c.scopes.push_child(span)?;
        expr(hir.iter, c, Needs::Value)?.apply(c)?;

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

    let binding_span = hir.binding.span();

    // Declare named loop variable.
    let binding_offset = {
        c.asm.push(Inst::unit(), hir.iter.span());
        c.scopes.decl_anon(binding_span)?
    };

    // Declare storage for memoized `next` instance fn.
    let next_offset = if c.options.memoize_instance_fn {
        let span = hir.iter.span();

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
        label: hir.label.copied(),
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
            hir.iter.span(),
            "copy iterator",
        );

        c.asm.push_with_comment(
            Inst::Copy {
                offset: next_offset,
            },
            hir.iter.span(),
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
            hir.iter.span(),
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

    let body_span = hir.body.span();
    let guard = c.scopes.push_child(body_span)?;

    pat_with_offset(hir.binding, c, binding_offset)?;

    block(hir.body, c, Needs::None)?.apply(c)?;
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
fn expr_if(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprIf<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let then_label = c.asm.new_label("if_then");
    let end_label = c.asm.new_label("if_end");

    let mut branches = Vec::new();
    let then_scope = condition(hir.condition, c, then_label)?;

    for branch in hir.expr_else_ifs {
        let label = c.asm.new_label("if_branch");
        let scope = condition(branch.condition, c, label)?;
        branches.push((branch, label, scope));
    }

    // use fallback as fall through.
    if let Some(fallback) = hir.expr_else {
        block(fallback.block, c, needs)?.apply(c)?;
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
    block(hir.block, c, needs)?.apply(c)?;
    c.clean_last_scope(span, expected, needs)?;

    if !hir.expr_else_ifs.is_empty() {
        c.asm.jump(end_label, span);
    }

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        let span = branch.span();

        c.asm.label(label)?;

        let scopes = c.scopes.push(scope);
        block(branch.block, c, needs)?.apply(c)?;
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
fn expr_index(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprIndex<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let guard = c.scopes.push_child(span)?;

    let target = expr(hir.target, c, Needs::Value)?.apply_targeted(c)?;
    let index = expr(hir.index, c, Needs::Value)?.apply_targeted(c)?;

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
fn expr_let(hir: &hir::ExprLet<'_>, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    let load = |c: &mut Assembler, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(hir.expr, c, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(hir.pat, c, false_label, &load)? {
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
fn expr_loop(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprLoop<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let continue_label = c.asm.new_label("loop_continue");
    let break_label = c.asm.new_label("loop_break");

    let var_count = c.scopes.total_var_count(span)?;

    let _guard = c.loops.push(Loop {
        label: hir.label.copied(),
        continue_label,
        continue_var_count: var_count,
        break_label,
        break_var_count: var_count,
        needs,
        drop: None,
    });

    c.asm.label(continue_label)?;
    block(hir.body, c, Needs::None)?.apply(c)?;
    c.asm.jump(continue_label, span);
    c.asm.label(break_label)?;

    Ok(Asm::top(span))
}

#[instrument]
fn expr_match(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprMatch<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let expected_scopes = c.scopes.push_child(span)?;

    expr(hir.expr, c, Needs::Value)?.apply(c)?;
    // Offset of the expression.
    let offset = c.scopes.decl_anon(span)?;

    let end_label = c.asm.new_label("match_end");
    let mut branches = Vec::new();

    for branch in hir.branches {
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

        pat(branch.pat, c, match_false, &load)?;

        let scope = if let Some(condition) = branch.condition {
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

    let mut it = hir.branches.iter().zip(&branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch.span();

        c.asm.label(*label)?;

        let expected = c.scopes.push(scope.clone());
        expr(branch.body, c, needs)?.apply(c)?;
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
fn expr_object(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprObject<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let guard = c.scopes.push_child(span)?;

    let mut keys = Vec::<Box<str>>::new();
    let mut check_keys = Vec::new();
    let mut keys_dup = HashMap::new();

    for assign in hir.assignments {
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

    for assign in hir.assignments {
        let span = assign.span();

        if let Some(e) = assign.assign {
            expr(e, c, Needs::Value)?.apply(c)?;
        } else {
            let key = assign.key.resolve(resolve_context!(c.q))?;
            let var = c
                .scopes
                .get_var(c.q.visitor, key.as_ref(), c.source_id, span)?;
            let comment = format!("name `{}`", key);
            var.copy(c, span, comment);
        }

        c.scopes.decl_anon(span)?;
    }

    let slot = c.q.unit.new_static_object_keys_iter(span, &keys)?;

    match hir.path {
        Some(path) => {
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
        None => {
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
fn path(hir: &hir::Path<'_>, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    if let Some(ast::PathKind::SelfValue) = hir.as_kind() {
        let var = c.scopes.get_var(c.q.visitor, SELF, c.source_id, span)?;

        if needs.value() {
            var.copy(c, span, SELF);
        }

        return Ok(Asm::top(span));
    }

    let named = c.convert_path(hir)?;

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
        // light heuristics, treat it as a type error in case the first
        // character is uppercase.
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
fn expr_range(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprRange<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let guard = c.scopes.push_child(span)?;

    if needs.value() {
        let from = if let Some(from) = hir.from {
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

        let to = if let Some(to) = hir.to {
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

        let limits = match hir.limits {
            hir::ExprRangeLimits::HalfOpen => InstRangeLimits::HalfOpen,
            hir::ExprRangeLimits::Closed => InstRangeLimits::Closed,
        };

        c.asm.push(Inst::Range { limits }, span);
        c.scopes.undecl_anon(span, 2)?;
    } else {
        if let Some(from) = hir.from {
            expr(from, c, needs)?.apply(c)?;
        }

        if let Some(to) = hir.to {
            expr(to, c, needs)?.apply(c)?;
        }
    }

    c.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a return expression.
#[instrument]
fn expr_return(
    span: Span,
    c: &mut Assembler<'_>,
    hir: Option<&hir::Expr<'_>>,
    _: Needs,
) -> CompileResult<Asm> {
    // NB: drop any loop temporaries.
    for l in c.loops.iter() {
        if let Some(offset) = l.drop {
            c.asm.push(Inst::Drop { offset }, span);
        }
    }

    if let Some(e) = hir {
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
fn expr_select(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprSelect<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let len = hir.branches.len();
    c.contexts.push(span);

    let mut default_branch = None;
    let mut branches = Vec::new();

    let end_label = c.asm.new_label("select_end");

    for branch in hir.branches {
        match *branch {
            hir::ExprSelectBranch::Pat(pat) => {
                let label = c.asm.new_label("select_branch");
                branches.push((label, pat));
            }
            hir::ExprSelectBranch::Default(def) => {
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
        expr(branch.expr, c, Needs::Value)?.apply(c)?;
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
            match branch.pat.kind {
                hir::PatKind::PatPath(path) => {
                    let named = c.convert_path(path)?;
                    named.assert_not_generic()?;

                    if let Some(local) = named.as_local() {
                        c.scopes.decl_var(local, path.span())?;
                        break;
                    }
                }
                hir::PatKind::PatIgnore => {
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
        expr(branch.body, c, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;
        c.asm.jump(end_label, span);
    }

    if let Some((branch, label)) = default_branch {
        c.asm.label(label)?;
        expr(branch, c, needs)?.apply(c)?;
    }

    c.asm.label(end_label)?;

    c.contexts
        .pop()
        .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

    Ok(Asm::top(span))
}

/// Assemble a try expression.
#[instrument]
fn expr_try(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::Expr<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let clean = c.scopes.total_var_count(span)?;
    let address = expr(hir, c, Needs::Value)?.apply_targeted(c)?;

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
fn expr_tuple(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprSeq<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    macro_rules! tuple {
        ($variant:ident, $c:ident, $span:expr, $($var:ident),*) => {{
            let guard = $c.scopes.push_child($span)?;

            let mut it = hir.items.iter();

            $(
            let $var = it.next().ok_or_else(|| CompileError::new($span, CompileErrorKind::Custom { message: "items ended unexpectedly" }))?;
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

    if hir.items.is_empty() {
        c.asm.push(Inst::unit(), span);
    } else {
        match hir.items.len() {
            1 => tuple!(Tuple1, c, span, e1),
            2 => tuple!(Tuple2, c, span, e1, e2),
            3 => tuple!(Tuple3, c, span, e1, e2, e3),
            4 => tuple!(Tuple4, c, span, e1, e2, e3, e4),
            _ => {
                for e in hir.items {
                    expr(e, c, Needs::Value)?.apply(c)?;
                    c.scopes.decl_anon(e.span())?;
                }

                c.asm.push(
                    Inst::Tuple {
                        count: hir.items.len(),
                    },
                    span,
                );

                c.scopes.undecl_anon(span, hir.items.len())?;
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
fn expr_unary(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprUnary<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    // NB: special unary expressions.
    if let ast::UnOp::BorrowRef { .. } = hir.op {
        return Err(CompileError::new(span, CompileErrorKind::UnsupportedRef));
    }

    if let (ast::UnOp::Neg(..), hir::ExprKind::Lit(ast::Lit::Number(n))) = (hir.op, hir.expr.kind) {
        match n.resolve(resolve_context!(c.q))? {
            ast::Number::Float(n) => {
                c.asm.push(Inst::float(-n), span);
            }
            ast::Number::Integer(int) => {
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

    expr(hir.expr, c, Needs::Value)?.apply(c)?;

    match hir.op {
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
fn expr_vec(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprSeq<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let count = hir.items.len();

    for e in hir.items {
        expr(e, c, Needs::Value)?.apply(c)?;
        c.scopes.decl_anon(e.span())?;
    }

    c.asm.push(Inst::Vec { count }, span);
    c.scopes.undecl_anon(span, hir.items.len())?;

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
fn expr_while(
    span: Span,
    c: &mut Assembler<'_>,
    hir: &hir::ExprWhile<'_>,
    needs: Needs,
) -> CompileResult<Asm> {
    let continue_label = c.asm.new_label("while_continue");
    let then_label = c.asm.new_label("whiel_then");
    let end_label = c.asm.new_label("while_end");
    let break_label = c.asm.new_label("while_break");

    let var_count = c.scopes.total_var_count(span)?;

    let _guard = c.loops.push(Loop {
        label: hir.label.copied(),
        continue_label,
        continue_var_count: var_count,
        break_label,
        break_var_count: var_count,
        needs,
        drop: None,
    });

    c.asm.label(continue_label)?;

    let then_scope = condition(hir.condition, c, then_label)?;
    let expected = c.scopes.push(then_scope);

    c.asm.jump(end_label, span);
    c.asm.label(then_label)?;

    block(hir.body, c, Needs::None)?.apply(c)?;
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
fn expr_yield(
    span: Span,
    c: &mut Assembler<'_>,
    hir: Option<&hir::Expr<'_>>,
    needs: Needs,
) -> CompileResult<Asm> {
    if let Some(e) = hir {
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

/// Assemble a function from an [hir::ItemFn<'_>].
#[instrument]
pub(crate) fn fn_from_item_fn(
    hir: &hir::ItemFn<'_>,
    c: &mut Assembler<'_>,
    instance_fn: bool,
) -> CompileResult<()> {
    let span = hir.span();

    let mut patterns = Vec::new();
    let mut first = true;

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(span) => {
                if !instance_fn || !first {
                    return Err(CompileError::new(*span, CompileErrorKind::UnsupportedSelf));
                }

                c.scopes.new_var(SELF, *span)?;
            }
            hir::FnArg::Pat(pat) => {
                let offset = c.scopes.decl_anon(pat.span())?;
                patterns.push((pat, offset));
            }
        }

        first = false;
    }

    for (pat, offset) in patterns {
        pat_with_offset(pat, c, offset)?;
    }

    if hir.body.statements.is_empty() {
        let total_var_count = c.scopes.total_var_count(span)?;
        c.locals_pop(total_var_count, span);
        c.asm.push(Inst::ReturnUnit, span);
        return Ok(());
    }

    if !hir.body.produces_nothing() {
        return_(c, span, hir.body, block)?;
    } else {
        block(hir.body, c, Needs::None)?.apply(c)?;

        let total_var_count = c.scopes.total_var_count(span)?;
        c.locals_pop(total_var_count, span);
        c.asm.push(Inst::ReturnUnit, span);
    }

    c.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a literal value.
#[instrument]
fn lit(hir: &ast::Lit, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    match hir {
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
            let slot = c.q.unit.new_static_bytes(span, bytes.as_ref())?;
            c.asm.push(Inst::Bytes { slot }, span);
        }
    };

    Ok(Asm::top(span))
}

#[instrument]
fn lit_str(hir: &ast::LitStr, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    let string = hir.resolve(resolve_context!(c.q))?;
    let slot = c.q.unit.new_static_string(span, string.as_ref())?;
    c.asm.push(Inst::String { slot }, span);
    Ok(Asm::top(span))
}

/// Assemble a literal number.
#[instrument]
fn lit_number(hir: &ast::LitNumber, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    // NB: don't encode unecessary literal.
    let number = hir.resolve(resolve_context!(c.q))?;

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
fn local(hir: &hir::Local<'_>, c: &mut Assembler<'_>, needs: Needs) -> CompileResult<Asm> {
    let span = hir.span();

    let load = |c: &mut Assembler, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(hir.expr, c, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(hir.pat, c, false_label, &load)? {
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
fn pat_items_count(items: &[hir::Pat<'_>]) -> Result<(bool, usize), CompileError> {
    let mut it = items.iter();

    let (is_open, mut count) = match it.next_back() {
        Some(pat) => {
            if matches!(pat.kind, hir::PatKind::PatRest) {
                (true, 0)
            } else {
                (false, 1)
            }
        }
        None => return Ok((false, 0)),
    };

    for pat in it {
        if let hir::PatKind::PatRest = pat.kind {
            return Err(CompileError::new(
                pat.span(),
                CompileErrorKind::UnsupportedPatternRest,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}

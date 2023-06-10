use core::mem::{replace, take};

use crate::no_std::prelude::*;

use crate::ast::{self, Span, Spanned};
use crate::compile::v1::{Assembler, Layer, Loop, Needs, Var};
use crate::compile::{self, CompileErrorKind, WithSpan};
use crate::hir;
use crate::parse::Resolve;
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRangeLimits, InstTarget, InstValue,
    InstVariant, Label, PanicReason, Protocol, TypeCheck,
};
use crate::Hash;

use rune_macros::instrument;

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
    fn top(span: &dyn Spanned) -> Self {
        Self {
            span: span.span(),
            kind: AsmKind::Top,
        }
    }

    fn var(span: &dyn Spanned, var: Var, local: Box<str>) -> Self {
        Self {
            span: span.span(),
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
    fn apply(self, c: &mut Assembler) -> compile::Result<()> {
        if let AsmKind::Var(var, local) = self.kind {
            var.copy(c, &self.span, format_args!("var `{}`", local));
        }

        Ok(())
    }

    /// Assemble into an instruction declaring an anonymous variable if appropriate.
    fn apply_targeted(self, c: &mut Assembler) -> compile::Result<InstAddress> {
        let address = match self.kind {
            AsmKind::Top => {
                c.scopes.alloc(&self.span)?;
                InstAddress::Top
            }
            AsmKind::Var(var, ..) => InstAddress::Offset(var.offset),
        };

        Ok(address)
    }
}

/// Assemble a function from an [hir::ItemFn<'_>].
#[instrument(span = hir)]
pub(crate) fn fn_from_item_fn<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ItemFn<'hir>,
    instance_fn: bool,
) -> compile::Result<()> {
    let mut patterns = Vec::new();
    let mut first = true;

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(span, variable) => {
                if !instance_fn || !first {
                    return Err(compile::Error::new(
                        *span,
                        CompileErrorKind::UnsupportedSelf,
                    ));
                }

                c.scopes.define(*variable, SELF, span)?;
            }
            hir::FnArg::Pat(pat) => {
                let offset = c.scopes.alloc(pat)?;
                patterns.push((pat, offset));
            }
        }

        first = false;
    }

    for (pat, offset) in patterns {
        pat_with_offset(c, pat, offset)?;
    }

    if hir.body.statements.is_empty() {
        let total_var_count = c.scopes.total(hir)?;
        c.locals_pop(total_var_count, hir);
        c.asm.push(Inst::ReturnUnit, hir);
        return Ok(());
    }

    if !hir.body.produces_nothing() {
        return_(c, hir, hir.body, block)?;
    } else {
        block(c, hir.body, Needs::None)?.apply(c)?;

        let total_var_count = c.scopes.total(hir)?;
        c.locals_pop(total_var_count, hir);
        c.asm.push(Inst::ReturnUnit, hir);
    }

    c.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an async block.
#[instrument(span = hir.block.span)]
pub(crate) fn async_block_secondary<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::AsyncBlock<'hir>,
) -> compile::Result<()> {
    for (variable, capture) in hir.captures.iter().copied() {
        let name = match capture {
            hir::Capture::SelfValue => SELF,
            hir::Capture::Name(name) => name,
        };

        c.scopes.define(variable, name, &hir.block)?;
    }

    return_(c, &hir.block, hir.block, block)?;
    c.scopes.pop_last(&hir.block)?;
    Ok(())
}

/// Assemble the body of a closure function.
#[instrument(span = span)]
pub(crate) fn expr_closure_secondary<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprClosure<'hir>,
    span: &dyn Spanned,
) -> compile::Result<()> {
    let mut patterns = Vec::new();

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(..) => {
                return Err(compile::Error::new(arg, CompileErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
                let offset = c.scopes.alloc(pat)?;
                patterns.push((pat, offset));
            }
        }
    }

    if !hir.captures.is_empty() {
        c.asm.push(Inst::PushTuple, span);

        for (variable, capture) in hir.captures.iter().copied() {
            let name = match capture {
                hir::Capture::SelfValue => SELF,
                hir::Capture::Name(name) => name,
            };

            c.scopes.define(variable, name, span)?;
        }
    }

    for (pat, offset) in patterns {
        pat_with_offset(c, pat, offset)?;
    }

    return_(c, span, hir.body, expr)?;
    c.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a return statement from the given Assemble.
fn return_<'hir, T>(
    c: &mut Assembler<'_, 'hir>,
    span: &dyn Spanned,
    hir: &T,
    asm: impl FnOnce(&mut Assembler<'_, 'hir>, &T, Needs) -> compile::Result<Asm>,
) -> compile::Result<()> {
    let clean = c.scopes.total(span)?;

    let address = asm(c, hir, Needs::Value)?.apply_targeted(c)?;
    c.asm.push(Inst::Return { address, clean }, span);

    // Top address produces an anonymous variable, which is consumed by the
    // return statement.
    if let InstAddress::Top = address {
        c.scopes.free(span, 1)?;
    }

    Ok(())
}

/// Compile a pattern based on the given offset.
#[instrument(span = hir)]
fn pat_with_offset<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Pat<'hir>,
    offset: usize,
) -> compile::Result<()> {
    let load = |c: &mut Assembler<'_, 'hir>, needs: Needs| {
        if needs.value() {
            c.asm.push(Inst::Copy { offset }, hir);
        }

        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(c, hir, &false_label, &load)? {
        c.q.diagnostics
            .let_pattern_might_panic(c.source_id, hir, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(&ok_label, hir);
        c.asm.label(&false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        );

        c.asm.label(&ok_label)?;
    }

    Ok(())
}

/// Encode a pattern.
///
/// Patterns will clean up their own locals and execute a jump to `false_label`
/// in case the pattern does not match.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Pat<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Assembler<'_, 'hir>, Needs) -> compile::Result<()>,
) -> compile::Result<bool> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have side effects, so must
            // call the load generator.
            load(c, Needs::None)?;
            Ok(false)
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                load(c, Needs::Value)?;
                c.asm.push(to_tuple_match_instruction(*kind), hir);
                c.asm
                    .pop_and_jump_if_not(c.scopes.local(hir)?, false_label, hir);
                Ok(true)
            }
            hir::PatPathKind::Ident(name, variable) => {
                load(c, Needs::Value)?;
                c.scopes.define(variable, name, hir)?;
                Ok(false)
            }
        },
        hir::PatKind::Lit(hir) => Ok(pat_lit(c, hir, false_label, load)?),
        hir::PatKind::Vec(hir) => {
            pat_vec(c, hir, span, false_label, &load)?;
            Ok(true)
        }
        hir::PatKind::Tuple(hir) => {
            pat_tuple(c, hir, span, false_label, &load)?;
            Ok(true)
        }
        hir::PatKind::Object(hir) => {
            pat_object(c, hir, span, false_label, &load)?;
            Ok(true)
        }
        _ => Err(compile::Error::new(
            hir,
            CompileErrorKind::UnsupportedPatternExpr,
        )),
    }
}

/// Assemble a pattern literal.
#[instrument(span = hir)]
fn pat_lit<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Expr<'_>,
    false_label: &Label,
    load: &dyn Fn(&mut Assembler<'_, 'hir>, Needs) -> compile::Result<()>,
) -> compile::Result<bool> {
    let Some(inst) = pat_lit_inst(c, hir)? else {
        return Err(compile::Error::new(
            hir,
            CompileErrorKind::UnsupportedPatternExpr,
        ));
    };

    load(c, Needs::Value)?;
    c.asm.push(inst, hir);
    c.asm
        .pop_and_jump_if_not(c.scopes.local(hir)?, false_label, hir);
    Ok(true)
}

#[instrument(span = hir)]
fn pat_lit_inst<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Expr<'_>,
) -> compile::Result<Option<Inst>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let inst = match lit {
        hir::Lit::Byte(byte) => Inst::EqByte { byte },
        hir::Lit::Char(char) => Inst::EqChar { char },
        hir::Lit::Str(string) => Inst::EqString {
            slot: c.q.unit.new_static_string(hir, string)?,
        },
        hir::Lit::ByteStr(bytes) => Inst::EqBytes {
            slot: c.q.unit.new_static_bytes(hir, bytes)?,
        },
        hir::Lit::Integer(integer) => Inst::EqInteger { integer },
        hir::Lit::Bool(boolean) => Inst::EqBool { boolean },
        _ => return Ok(None),
    };

    Ok(Some(inst))
}

/// Assemble an [hir::Condition<'_>].
#[instrument(span = condition)]
fn condition<'hir>(
    c: &mut Assembler<'_, 'hir>,
    condition: &hir::Condition<'hir>,
    then_label: &Label,
) -> compile::Result<Layer<'hir>> {
    match condition {
        hir::Condition::Expr(e) => {
            let guard = c.scopes.child(e)?;
            expr(c, e, Needs::Value)?.apply(c)?;
            c.asm.jump_if(then_label, e);
            Ok(c.scopes.pop(guard, e)?)
        }
        hir::Condition::ExprLet(expr_let) => {
            let span = expr_let;

            let false_label = c.asm.new_label("if_condition_false");

            let expected = c.scopes.child(span)?;

            let load = |c: &mut Assembler<'_, 'hir>, needs: Needs| {
                expr(c, expr_let.expr, needs)?.apply(c)?;
                Ok(())
            };

            if pat(c, expr_let.pat, &false_label, &load)? {
                c.asm.jump(then_label, span);
                c.asm.label(&false_label)?;
            } else {
                c.asm.jump(then_label, span);
            };

            Ok(c.scopes.pop(expected, span)?)
        }
    }
}

/// Encode a vector pattern match.
#[instrument(span = span)]
fn pat_vec<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::PatItems<'hir>,
    span: &dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Assembler<'_, 'hir>, Needs) -> compile::Result<()>,
) -> compile::Result<()> {
    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    load(c, Needs::Value)?;
    let offset = c.scopes.alloc(span)?;

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    c.asm.push(Inst::Copy { offset }, span);

    c.asm.push(
        Inst::MatchSequence {
            type_check: TypeCheck::Vec,
            len: hir.count,
            exact: !hir.is_open,
        },
        span,
    );

    c.asm
        .pop_and_jump_if_not(c.scopes.local(span)?, false_label, span);

    for (index, hir) in hir.items.iter().take(hir.count).enumerate() {
        let load = move |c: &mut Assembler<'_, 'hir>, needs: Needs| {
            if needs.value() {
                c.asm.push(Inst::TupleIndexGetAt { offset, index }, hir);
            }

            Ok(())
        };

        pat(c, hir, false_label, &load)?;
    }

    Ok(())
}

/// Encode a vector pattern match.
#[instrument(span = span)]
fn pat_tuple<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::PatItems<'hir>,
    span: &dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Assembler<'_, 'hir>, Needs) -> compile::Result<()>,
) -> compile::Result<()> {
    load(c, Needs::Value)?;

    if hir.items.is_empty() {
        c.asm.push(Inst::IsUnit, span);

        c.asm
            .pop_and_jump_if_not(c.scopes.local(span)?, false_label, span);
        return Ok(());
    }

    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    let offset = c.scopes.alloc(span)?;

    let inst = to_tuple_match_instruction(hir.kind);

    c.asm.push(Inst::Copy { offset }, span);
    c.asm.push(inst, span);

    c.asm
        .pop_and_jump_if_not(c.scopes.local(span)?, false_label, span);

    for (index, p) in hir.items.iter().take(hir.count).enumerate() {
        let load = move |c: &mut Assembler<'_, 'hir>, needs: Needs| {
            if needs.value() {
                c.asm.push(Inst::TupleIndexGetAt { offset, index }, p);
            }

            Ok(())
        };

        pat(c, p, false_label, &load)?;
    }

    Ok(())
}

fn to_tuple_match_instruction(kind: hir::PatItemsKind) -> Inst {
    match kind {
        hir::PatItemsKind::Type { hash } => Inst::MatchType { hash },
        hir::PatItemsKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn { type_check },
        hir::PatItemsKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
        },
        hir::PatItemsKind::Anonymous { count, is_open } => Inst::MatchSequence {
            type_check: TypeCheck::Tuple,
            len: count,
            exact: !is_open,
        },
    }
}

/// Assemble an object pattern.
#[instrument(span = span)]
fn pat_object<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::PatItems<'hir>,
    span: &dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Assembler<'_, 'hir>, Needs) -> compile::Result<()>,
) -> compile::Result<()> {
    // NB: bind the loaded variable (once) to an anonymous var.
    // We reduce the number of copy operations by having specialized
    // operations perform the load from the given offset.
    load(c, Needs::Value)?;
    let offset = c.scopes.alloc(span)?;

    let mut string_slots = Vec::new();

    for binding in hir.bindings {
        string_slots.push(c.q.unit.new_static_string(span, binding.key())?);
    }

    let inst = match hir.kind {
        hir::PatItemsKind::Type { hash } => Inst::MatchType { hash },
        hir::PatItemsKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn { type_check },
        hir::PatItemsKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
        },
        hir::PatItemsKind::Anonymous { is_open, .. } => {
            let keys =
                c.q.unit
                    .new_static_object_keys_iter(span, hir.bindings.iter().map(|b| b.key()))?;

            Inst::MatchObject {
                slot: keys,
                exact: !is_open,
            }
        }
    };

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    c.asm.push(Inst::Copy { offset }, span);
    c.asm.push(inst, span);

    c.asm
        .pop_and_jump_if_not(c.scopes.local(span)?, false_label, span);

    for (binding, slot) in hir.bindings.iter().zip(string_slots) {
        match *binding {
            hir::Binding::Binding(span, _, p) => {
                let load = move |c: &mut Assembler<'_, 'hir>, needs: Needs| {
                    if needs.value() {
                        c.asm.push(Inst::ObjectIndexGetAt { offset, slot }, &span);
                    }

                    Ok(())
                };

                pat(c, p, false_label, &load)?;
            }
            hir::Binding::Ident(span, name, variable) => {
                c.asm.push(Inst::ObjectIndexGetAt { offset, slot }, &span);
                c.scopes.define(variable, name, &span)?;
            }
        }
    }

    Ok(())
}

/// Call a block.
#[instrument(span = hir)]
fn block<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Block<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    c.contexts.push(hir.span());
    let scopes_count = c.scopes.child(hir)?;

    let mut last = None::<(&hir::Expr<'_>, bool)>;

    for stmt in hir.statements {
        let (e, semi) = match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = take(&mut last) {
                    // NB: terminated expressions do not need to produce a value.
                    expr(c, e, Needs::None)?.apply(c)?;
                }

                local(c, l, Needs::None)?.apply(c)?;
                continue;
            }
            hir::Stmt::Expr(expr) => (expr, false),
            hir::Stmt::Semi(expr) => (expr, true),
            hir::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = replace(&mut last, Some((e, semi))) {
            // NB: terminated expressions do not need to produce a value.
            expr(c, e, Needs::None)?.apply(c)?;
        }
    }

    let produced = if let Some((e, semi)) = last {
        if semi {
            expr(c, e, Needs::None)?.apply(c)?;
            false
        } else {
            expr(c, e, needs)?.apply(c)?;
            true
        }
    } else {
        false
    };

    let scope = c.scopes.pop(scopes_count, hir)?;

    if needs.value() {
        if produced {
            c.locals_clean(scope.local, hir);
        } else {
            c.locals_pop(scope.local, hir);
            c.asm.push(Inst::unit(), hir);
        }
    } else {
        c.locals_pop(scope.local, hir);
    }

    c.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    Ok(Asm::top(hir))
}

/// Assemble #[builtin] format_args!(...) macro.
#[instrument(span = format)]
fn builtin_format<'hir>(
    c: &mut Assembler<'_, 'hir>,
    format: &hir::BuiltInFormat<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    use crate::runtime::format;

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

    expr(c, format.value, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Format { spec }, format);

    if !needs.value() {
        c.asm.push(Inst::Pop, format);
    }

    Ok(Asm::top(format))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument(span = template)]
fn builtin_template<'hir>(
    c: &mut Assembler<'_, 'hir>,
    template: &hir::BuiltInTemplate<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    let span = template;

    let expected = c.scopes.child(span)?;
    let mut size_hint = 0;
    let mut expansions = 0;

    for hir in template.exprs {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = hir.kind {
            size_hint += s.len();
            let slot = c.q.unit.new_static_string(span, s)?;
            c.asm.push(Inst::String { slot }, span);
            c.scopes.alloc(span)?;
            continue;
        }

        expansions += 1;

        expr(c, hir, Needs::Value)?.apply(c)?;
        c.scopes.alloc(span)?;
    }

    if template.from_literal && expansions == 0 {
        c.q.diagnostics
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
#[instrument(span = span)]
fn const_<'hir>(
    c: &mut Assembler<'_, 'hir>,
    value: &ConstValue,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<()> {
    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
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
            c.asm.push(Inst::integer(*n), span);
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
                const_(c, value, span, Needs::Value)?;
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
                const_(c, value, span, Needs::Value)?;
            }

            c.asm.push(Inst::Vec { count: vec.len() }, span);
        }
        ConstValue::Tuple(tuple) => {
            for value in tuple.iter() {
                const_(c, value, span, Needs::Value)?;
            }

            c.asm.push(Inst::Tuple { count: tuple.len() }, span);
        }
        ConstValue::Object(object) => {
            let mut entries = object.iter().collect::<Vec<_>>();
            entries.sort_by_key(|k| k.0);

            for (_, value) in entries.iter().copied() {
                const_(c, value, span, Needs::Value)?;
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
#[instrument(span = hir)]
fn expr<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Expr<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    let span = hir;

    let asm = match hir.kind {
        hir::ExprKind::SelfValue(variable) => {
            let var = c
                .scopes
                .get(c.q.visitor, variable, SELF, c.source_id, span)?;

            if needs.value() {
                var.copy(c, span, SELF);
            }

            Asm::top(span)
        }
        hir::ExprKind::Variable(variable, name) => {
            let var = c
                .scopes
                .get(c.q.visitor, variable, name, c.source_id, span)?;
            Asm::var(span, var, name.into())
        }
        hir::ExprKind::Type(ty) => {
            c.asm.push(
                Inst::Push {
                    value: InstValue::Type(ty),
                },
                span,
            );
            Asm::top(span)
        }
        hir::ExprKind::Fn(hash) => {
            c.asm.push(Inst::LoadFn { hash }, span);
            Asm::top(span)
        }
        hir::ExprKind::For(hir) => expr_for(c, hir, span, needs)?,
        hir::ExprKind::Loop(hir) => expr_loop(c, hir, span, needs)?,
        hir::ExprKind::Let(hir) => expr_let(c, hir, needs)?,
        hir::ExprKind::Group(hir) => expr(c, hir, needs)?,
        hir::ExprKind::Unary(hir) => expr_unary(c, hir, span, needs)?,
        hir::ExprKind::Assign(hir) => expr_assign(c, hir, span, needs)?,
        hir::ExprKind::Binary(hir) => expr_binary(c, hir, span, needs)?,
        hir::ExprKind::If(hir) => expr_if(c, hir, span, needs)?,
        hir::ExprKind::Index(hir) => expr_index(c, hir, span, needs)?,
        hir::ExprKind::Break(hir) => expr_break(c, hir, span, needs)?,
        hir::ExprKind::Continue(hir) => expr_continue(c, hir, span, needs)?,
        hir::ExprKind::Yield(hir) => expr_yield(c, hir, span, needs)?,
        hir::ExprKind::Block(hir) => block(c, hir, needs)?,
        hir::ExprKind::Return(hir) => expr_return(c, hir, span, needs)?,
        hir::ExprKind::Match(hir) => expr_match(c, hir, span, needs)?,
        hir::ExprKind::Await(hir) => expr_await(c, hir, span, needs)?,
        hir::ExprKind::Try(hir) => expr_try(c, hir, span, needs)?,
        hir::ExprKind::Select(hir) => expr_select(c, hir, span, needs)?,
        hir::ExprKind::Call(hir) => expr_call(c, hir, span, needs)?,
        hir::ExprKind::FieldAccess(hir) => expr_field_access(c, hir, span, needs)?,
        hir::ExprKind::CallClosure(hir) => expr_call_closure(c, hir, span, needs)?,
        hir::ExprKind::Lit(hir) => lit(c, hir, span, needs)?,
        hir::ExprKind::Tuple(hir) => expr_tuple(c, hir, span, needs)?,
        hir::ExprKind::Vec(hir) => expr_vec(c, hir, span, needs)?,
        hir::ExprKind::Object(hir) => expr_object(c, hir, span, needs)?,
        hir::ExprKind::Range(hir) => expr_range(c, hir, span, needs)?,
        hir::ExprKind::Template(template) => builtin_template(c, template, needs)?,
        hir::ExprKind::Format(format) => builtin_format(c, format, needs)?,
        hir::ExprKind::AsyncBlock(hir) => expr_async_block(c, hir, span, needs)?,
        hir::ExprKind::Const(id) => const_item(c, id, span, needs)?,
        hir::ExprKind::Path(path) => {
            return Err(compile::Error::msg(
                path,
                "Path expression is not supported here",
            ))
        }
    };

    Ok(asm)
}

/// Assemble an assign expression.
#[instrument(span = span)]
fn expr_assign<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprAssign<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Variable(variable, name) => {
            expr(c, hir.rhs, Needs::Value)?.apply(c)?;
            let var = c
                .scopes
                .get(c.q.visitor, variable, name, c.source_id, span)?;
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
                        let slot = c.q.unit.new_static_string(ident, slot.as_ref())?;

                        expr(c, hir.rhs, Needs::Value)?.apply(c)?;
                        c.scopes.alloc(hir.rhs)?;

                        expr(c, field_access.expr, Needs::Value)?.apply(c)?;
                        c.scopes.alloc(span)?;

                        c.asm.push(Inst::ObjectIndexSet { slot }, span);
                        c.scopes.free(span, 2)?;
                        true
                    } else {
                        false
                    }
                }
                hir::ExprField::LitNumber(field) => {
                    let number = field.resolve(resolve_context!(c.q))?;
                    let index = number.as_tuple_index().ok_or_else(|| {
                        compile::Error::new(
                            span,
                            CompileErrorKind::UnsupportedTupleIndex { number },
                        )
                    })?;

                    expr(c, hir.rhs, Needs::Value)?.apply(c)?;
                    c.scopes.alloc(hir.rhs)?;

                    expr(c, field_access.expr, Needs::Value)?.apply(c)?;
                    c.asm.push(Inst::TupleIndexSet { index }, span);
                    c.scopes.free(span, 1)?;
                    true
                }
            }
        }
        hir::ExprKind::Index(expr_index_get) => {
            expr(c, hir.rhs, Needs::Value)?.apply(c)?;
            c.scopes.alloc(span)?;

            expr(c, expr_index_get.target, Needs::Value)?.apply(c)?;
            c.scopes.alloc(span)?;

            expr(c, expr_index_get.index, Needs::Value)?.apply(c)?;
            c.scopes.alloc(span)?;

            c.asm.push(Inst::IndexSet, span);
            c.scopes.free(span, 3)?;
            true
        }
        _ => false,
    };

    if !supported {
        return Err(compile::Error::new(
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
#[instrument(span = hir)]
fn expr_await<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Expr<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    expr(c, hir, Needs::Value)?.apply(c)?;
    c.asm.push(Inst::Await, span);

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a binary expression.
#[instrument(span = span)]
fn expr_binary<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprBinary<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        compile_assign_binop(c, hir.lhs, hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::top(span));
    }

    if hir.op.is_conditional() {
        compile_conditional_binop(c, hir.lhs, hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::top(span));
    }

    let guard = c.scopes.child(span)?;

    // NB: need to declare these as anonymous local variables so that they
    // get cleaned up in case there is an early break (return, try, ...).
    let rhs_needs = rhs_needs_of(&hir.op);
    let a = expr(c, hir.lhs, Needs::Value)?.apply_targeted(c)?;
    let b = expr(c, hir.rhs, rhs_needs)?.apply_targeted(c)?;

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
            return Err(compile::Error::new(
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

    fn compile_conditional_binop<'hir>(
        c: &mut Assembler<'_, 'hir>,
        lhs: &hir::Expr<'hir>,
        rhs: &hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: Needs,
    ) -> compile::Result<()> {
        let end_label = c.asm.new_label("conditional_end");

        expr(c, lhs, Needs::Value)?.apply(c)?;

        match bin_op {
            ast::BinOp::And(..) => {
                c.asm.jump_if_not_or_pop(&end_label, lhs);
            }
            ast::BinOp::Or(..) => {
                c.asm.jump_if_or_pop(&end_label, lhs);
            }
            op => {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::UnsupportedBinaryOp { op: *op },
                ));
            }
        }

        expr(c, rhs, Needs::Value)?.apply(c)?;

        c.asm.label(&end_label)?;

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }

    fn compile_assign_binop<'hir>(
        c: &mut Assembler<'_, 'hir>,
        lhs: &hir::Expr<'hir>,
        rhs: &hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: Needs,
    ) -> compile::Result<()> {
        let supported = match lhs.kind {
            // <var> <op> <expr>
            hir::ExprKind::Variable(variable, name) => {
                expr(c, rhs, Needs::Value)?.apply(c)?;
                let var = c
                    .scopes
                    .get(c.q.visitor, variable, name, c.source_id, lhs)?;
                Some(InstTarget::Offset(var.offset))
            }
            // <expr>.<field> <op> <value>
            hir::ExprKind::FieldAccess(field_access) => {
                expr(c, field_access.expr, Needs::Value)?.apply(c)?;
                expr(c, rhs, Needs::Value)?.apply(c)?;

                // field assignment
                match field_access.expr_field {
                    hir::ExprField::Path(path) => {
                        if let Some(ident) = path.try_as_ident() {
                            let n = ident.resolve(resolve_context!(c.q))?;
                            let n = c.q.unit.new_static_string(path, n.as_ref())?;

                            Some(InstTarget::Field(n))
                        } else {
                            None
                        }
                    }
                    hir::ExprField::LitNumber(field) => {
                        let number = field.resolve(resolve_context!(c.q))?;

                        let Some(index) = number.as_tuple_index() else {
                            return Err(compile::Error::new(
                                field,
                                CompileErrorKind::UnsupportedTupleIndex { number },
                            ));
                        };

                        Some(InstTarget::TupleField(index))
                    }
                }
            }
            _ => None,
        };

        let Some(target) = supported else {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::UnsupportedBinaryExpr,
            ));
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
                return Err(compile::Error::new(
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
#[instrument(span = span)]
fn expr_async_block<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprAsyncBlock<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    for (variable, capture) in hir.captures.iter().copied() {
        let name = match capture {
            hir::Capture::SelfValue => SELF,
            hir::Capture::Name(name) => name,
        };

        if hir.do_move {
            let var = c
                .scopes
                .take(c.q.visitor, variable, name, c.source_id, span)?;
            var.do_move(c.asm, span, format_args!("captures `{}`", name));
        } else {
            let var = c
                .scopes
                .get(c.q.visitor, variable, name, c.source_id, span)?;
            var.copy(c, span, format_args!("captures `{}`", name));
        }
    }

    c.asm.push_with_comment(
        Inst::Call {
            hash: hir.hash,
            args: hir.captures.len(),
        },
        span,
        "async block",
    );

    if !needs.value() {
        c.asm
            .push_with_comment(Inst::Pop, span, "value is not needed");
    }

    Ok(Asm::top(span))
}

/// Assemble a constant item.
#[instrument(span = span)]
fn const_item<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hash: Hash,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let Some(const_value) = c.q.get_const_value(hash).cloned() else {
        return Err(compile::Error::msg(
            span,
            format_args!("Missing constant value for hash {hash}"),
        ));
    };

    const_(c, &const_value, span, needs)?;
    Ok(Asm::top(span))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument(span = span)]
fn expr_break<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: Option<&hir::ExprBreakValue<'hir>>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm> {
    let Some(current_loop) = c.loops.last() else {
        return Err(compile::Error::new(
            span,
            CompileErrorKind::BreakOutsideOfLoop,
        ));
    };

    let (last_loop, to_drop, has_value) = if let Some(e) = hir {
        match e {
            hir::ExprBreakValue::Expr(e) => {
                expr(c, e, current_loop.needs)?.apply(c)?;
                let to_drop = current_loop.drop.into_iter().collect();
                (current_loop, to_drop, true)
            }
            hir::ExprBreakValue::Label(label) => {
                let (last_loop, to_drop) =
                    c.loops.walk_until_label(resolve_context!(c.q), label)?;
                (last_loop, to_drop, false)
            }
        }
    } else {
        let to_drop = current_loop.drop.into_iter().collect();
        (current_loop, to_drop, false)
    };

    // Drop loop temporaries. Typically an iterator.
    for offset in to_drop {
        c.asm.push(Inst::Drop { offset }, span);
    }

    let vars = c
        .scopes
        .total(span)?
        .checked_sub(last_loop.break_var_count)
        .ok_or("Var count should be larger")
        .with_span(span)?;

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

    c.asm.jump(&last_loop.break_label, span);
    Ok(Asm::top(span))
}

/// Assemble a call expression.
#[instrument(span = span)]
fn expr_call<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprCall<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let args = hir.args.len();

    match hir.call {
        hir::Call::Var { name, variable, .. } => {
            let var = c
                .scopes
                .get(c.q.visitor, variable, name, c.source_id, span)?;

            for e in hir.args {
                expr(c, e, Needs::Value)?.apply(c)?;
                c.scopes.alloc(span)?;
            }

            var.copy(c, span, format_args!("var `{}`", name));
            c.scopes.alloc(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.free(span, hir.args.len() + 1)?;
        }
        hir::Call::Instance { target, hash } => {
            expr(c, target, Needs::Value)?.apply(c)?;
            c.scopes.alloc(target)?;

            for e in hir.args {
                expr(c, e, Needs::Value)?.apply(c)?;
                c.scopes.alloc(span)?;
            }

            c.asm.push(Inst::CallInstance { hash, args }, span);
            c.scopes.free(span, hir.args.len() + 1)?;
        }
        hir::Call::Meta { hash } => {
            for e in hir.args {
                expr(c, e, Needs::Value)?.apply(c)?;
                c.scopes.alloc(span)?;
            }

            c.asm.push(Inst::Call { hash, args }, span);
            c.scopes.free(span, args)?;
        }
        hir::Call::Expr { expr: e } => {
            for e in hir.args {
                expr(c, e, Needs::Value)?.apply(c)?;
                c.scopes.alloc(span)?;
            }

            expr(c, e, Needs::Value)?.apply(c)?;
            c.scopes.alloc(span)?;

            c.asm.push(Inst::CallFn { args }, span);

            c.scopes.free(span, args + 1)?;
        }
        hir::Call::ConstFn { id, ast_id } => {
            let const_fn = c.q.const_fn_for(id).with_span(span)?;
            let from = c.q.item_for(ast_id).with_span(span)?;
            let value = c.call_const_fn(span, &from, &const_fn, hir.args)?;
            const_(c, &value, span, Needs::Value)?;
        }
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a closure expression.
#[instrument(span = span)]
fn expr_call_closure<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprCallClosure<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    tracing::trace!(?hir.captures, "assemble call closure");

    // Construct a closure environment.
    for (variable, capture) in hir.captures.iter().copied() {
        let name = match capture {
            hir::Capture::SelfValue => SELF,
            hir::Capture::Name(name) => name,
        };

        if hir.do_move {
            let var = c
                .scopes
                .take(c.q.visitor, variable, name, c.source_id, span)?;
            var.do_move(c.asm, span, format_args!("capture `{}`", name));
        } else {
            let var = c
                .scopes
                .get(c.q.visitor, variable, name, c.source_id, span)?;
            var.copy(c, span, format_args!("capture `{}`", name));
        }
    }

    c.asm.push(
        Inst::Closure {
            hash: hir.hash,
            count: hir.captures.len(),
        },
        span,
    );

    Ok(Asm::top(span))
}

/// Assemble a continue expression.
#[instrument(span = span)]
fn expr_continue<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: Option<&ast::Label>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm> {
    let Some(current_loop) = c.loops.last() else {
        return Err(compile::Error::new(
            span,
            CompileErrorKind::ContinueOutsideOfLoop,
        ));
    };

    let last_loop = if let Some(label) = hir {
        let (last_loop, _) = c.loops.walk_until_label(resolve_context!(c.q), label)?;
        last_loop
    } else {
        current_loop
    };

    let vars = c
        .scopes
        .total(span)?
        .checked_sub(last_loop.continue_var_count)
        .ok_or("Var count should be larger")
        .with_span(span)?;

    c.locals_pop(vars, span);

    c.asm.jump(&last_loop.continue_label, span);
    Ok(Asm::top(span))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument(span = span)]
fn expr_field_access<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprFieldAccess<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    // Optimizations!
    //
    // TODO: perform deferred compilation for expressions instead, so we can
    // e.g. inspect if it compiles down to a local access instead of
    // climbing the hir like we do here.
    #[allow(clippy::single_match)]
    match (hir.expr.kind, hir.expr_field) {
        (hir::ExprKind::Variable(variable, name), hir::ExprField::LitNumber(n)) => {
            if try_immediate_field_access_optimization(c, variable, name, n, span, needs)? {
                return Ok(Asm::top(span));
            }
        }
        _ => (),
    }

    expr(c, hir.expr, Needs::Value)?.apply(c)?;

    match hir.expr_field {
        hir::ExprField::LitNumber(n) => {
            if let Some(index) = n.resolve(resolve_context!(c.q))?.as_tuple_index() {
                c.asm.push(Inst::TupleIndexGet { index }, span);

                if !needs.value() {
                    c.q.diagnostics.not_used(c.source_id, span, c.context());
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
                    c.q.diagnostics.not_used(c.source_id, span, c.context());
                    c.asm.push(Inst::Pop, span);
                }

                return Ok(Asm::top(span));
            }
        }
    }

    return Err(compile::Error::new(span, CompileErrorKind::BadFieldAccess));

    fn try_immediate_field_access_optimization<'hir>(
        c: &mut Assembler<'_, 'hir>,
        variable: hir::Variable,
        name: &'hir str,
        n: &ast::LitNumber,
        span: &dyn Spanned,
        needs: Needs,
    ) -> compile::Result<bool> {
        let ast::Number::Integer(index) = n.resolve(resolve_context!(c.q))? else {
            return Ok(false);
        };

        let Ok(index) = usize::try_from(index) else {
            return Ok(false);
        };

        let var = c
            .scopes
            .get(c.q.visitor, variable, name, c.source_id, span)?;

        c.asm.push(
            Inst::TupleIndexGetAt {
                offset: var.offset,
                index,
            },
            span,
        );

        if !needs.value() {
            c.q.diagnostics.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
        }

        Ok(true)
    }
}

/// Assemble an expression for loop.
#[instrument(span = span)]
fn expr_for<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprFor<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let continue_label = c.asm.new_label("for_continue");
    let end_label = c.asm.new_label("for_end");
    let break_label = c.asm.new_label("for_break");

    let break_var_count = c.scopes.total(span)?;

    let (iter_offset, loop_scope_expected) = {
        let loop_scope_expected = c.scopes.child(span)?;
        expr(c, hir.iter, Needs::Value)?.apply(c)?;

        let iter_offset = c.scopes.alloc(span)?;
        c.asm.push_with_comment(
            Inst::CallInstance {
                hash: *Protocol::INTO_ITER,
                args: 0,
            },
            span,
            format_args!("into_iter (offset: {})", iter_offset),
        );

        (iter_offset, loop_scope_expected)
    };

    // Declare named loop variable.
    let binding_offset = {
        c.asm.push(Inst::unit(), hir.iter);
        c.scopes.alloc(hir.binding)?
    };

    // Declare storage for memoized `next` instance fn.
    let next_offset = if c.options.memoize_instance_fn {
        let offset = c.scopes.alloc(hir.iter)?;

        // Declare the named loop variable and put it in the scope.
        c.asm.push_with_comment(
            Inst::Copy {
                offset: iter_offset,
            },
            hir.iter,
            "copy iterator (memoize)",
        );

        c.asm.push_with_comment(
            Inst::LoadInstanceFn {
                hash: *Protocol::NEXT,
            },
            hir.iter,
            "load instance fn (memoize)",
        );

        Some(offset)
    } else {
        None
    };

    let continue_var_count = c.scopes.total(span)?;
    c.asm.label(&continue_label)?;

    let _guard = c.loops.push(Loop {
        label: hir.label.copied(),
        continue_label: continue_label.clone(),
        continue_var_count,
        break_label: break_label.clone(),
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
            hir.iter,
            "copy iterator",
        );

        c.asm.push_with_comment(
            Inst::Copy {
                offset: next_offset,
            },
            hir.iter,
            "copy next",
        );

        c.asm.push(Inst::CallFn { args: 1 }, span);

        c.asm.push(
            Inst::Replace {
                offset: binding_offset,
            },
            hir.binding,
        );
    } else {
        // call the `next` function to get the next level of iteration, bind the
        // result to the loop variable in the loop.
        c.asm.push(
            Inst::Copy {
                offset: iter_offset,
            },
            hir.iter,
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
            hir.binding,
        );
    }

    // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
    c.asm.iter_next(binding_offset, &end_label, hir.binding);

    let guard = c.scopes.child(hir.body)?;

    pat_with_offset(c, hir.binding, binding_offset)?;

    block(c, hir.body, Needs::None)?.apply(c)?;
    c.clean_last_scope(span, guard, Needs::None)?;

    c.asm.jump(&continue_label, span);
    c.asm.label(&end_label)?;

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
    c.asm.label(&break_label)?;
    Ok(Asm::top(span))
}

/// Assemble an if expression.
#[instrument(span = span)]
fn expr_if<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Conditional<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let end_label = c.asm.new_label("if_end");

    let mut branches = Vec::new();
    let mut fallback = None;

    for branch in hir.branches {
        if fallback.is_some() {
            continue;
        }

        let Some(cond) = branch.condition else {
            fallback = Some(branch.block);
            continue;
        };

        let label = c.asm.new_label("if_branch");
        let scope = condition(c, cond, &label)?;
        branches.push((branch, label, scope));
    }

    // use fallback as fall through.
    if let Some(b) = fallback {
        block(c, b, needs)?.apply(c)?;
    } else {
        // NB: if we must produce a value and there is no fallback branch,
        // encode the result of the statement as a unit.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }
    }

    c.asm.jump(&end_label, span);

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        c.asm.label(&label)?;

        let scopes = c.scopes.push(scope);
        block(c, branch.block, needs)?.apply(c)?;
        c.clean_last_scope(branch, scopes, needs)?;

        if it.peek().is_some() {
            c.asm.jump(&end_label, branch);
        }
    }

    c.asm.label(&end_label)?;
    Ok(Asm::top(span))
}

/// Assemble an expression.
#[instrument(span = span)]
fn expr_index<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprIndex<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let guard = c.scopes.child(span)?;

    let target = expr(c, hir.target, Needs::Value)?.apply_targeted(c)?;
    let index = expr(c, hir.index, Needs::Value)?.apply_targeted(c)?;

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
#[instrument(span = hir)]
fn expr_let<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprLet<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    let load = |c: &mut Assembler<'_, 'hir>, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(c, hir.expr, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(c, hir.pat, &false_label, &load)? {
        c.q.diagnostics
            .let_pattern_might_panic(c.source_id, hir, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(&ok_label, hir);
        c.asm.label(&false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        );

        c.asm.label(&ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        c.asm.push(Inst::unit(), hir);
    }

    Ok(Asm::top(hir))
}

#[instrument(span = span)]
fn expr_match<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprMatch<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let expected_scopes = c.scopes.child(span)?;

    expr(c, hir.expr, Needs::Value)?.apply(c)?;
    // Offset of the expression.
    let offset = c.scopes.alloc(span)?;

    let end_label = c.asm.new_label("match_end");
    let mut branches = Vec::new();

    for branch in hir.branches {
        let span = branch;

        let branch_label = c.asm.new_label("match_branch");
        let match_false = c.asm.new_label("match_false");

        let parent_guard = c.scopes.child(span)?;

        let load = move |this: &mut Assembler, needs: Needs| {
            if needs.value() {
                this.asm.push(Inst::Copy { offset }, span);
            }

            Ok(())
        };

        pat(c, branch.pat, &match_false, &load)?;

        let scope = if let Some(condition) = branch.condition {
            let span = condition;

            let guard = c.scopes.child(span)?;

            expr(c, condition, Needs::Value)?.apply(c)?;
            c.clean_last_scope(span, guard, Needs::Value)?;
            let scope = c.scopes.pop(parent_guard, span)?;

            c.asm.pop_and_jump_if_not(scope.local, &match_false, span);

            c.asm.jump(&branch_label, span);
            scope
        } else {
            c.scopes.pop(parent_guard, span)?
        };

        c.asm.jump(&branch_label, span);
        c.asm.label(&match_false)?;

        branches.push((branch_label, scope));
    }

    // what to do in case nothing matches and the pattern doesn't have any
    // default match branch.
    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    c.asm.jump(&end_label, span);

    let mut it = hir.branches.iter().zip(&branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch;

        c.asm.label(label)?;

        let expected = c.scopes.push(scope.clone());
        expr(c, branch.body, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;

        if it.peek().is_some() {
            c.asm.jump(&end_label, span);
        }
    }

    c.asm.label(&end_label)?;

    // pop the implicit scope where we store the anonymous match variable.
    c.clean_last_scope(span, expected_scopes, needs)?;
    Ok(Asm::top(span))
}

/// Compile a literal object.
#[instrument(span = span)]
fn expr_object<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprObject<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let guard = c.scopes.child(span)?;

    for assign in hir.assignments {
        expr(c, assign.assign, Needs::Value)?.apply(c)?;
        c.scopes.alloc(&span)?;
    }

    let slot =
        c.q.unit
            .new_static_object_keys_iter(span, hir.assignments.iter().map(|a| a.key.1))?;

    match hir.kind {
        hir::ExprObjectKind::UnitStruct { hash } => {
            c.asm.push(Inst::UnitStruct { hash }, span);
        }
        hir::ExprObjectKind::Struct { hash } => {
            c.asm.push(Inst::Struct { hash, slot }, span);
        }
        hir::ExprObjectKind::StructVariant { hash } => {
            c.asm.push(Inst::StructVariant { hash, slot }, span);
        }
        hir::ExprObjectKind::Anonymous => {
            c.asm.push(Inst::Object { slot }, span);
        }
    }

    // No need to encode an object since the value is not needed.
    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    c.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a range expression.
#[instrument(span = span)]
fn expr_range<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprRange<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let guard = c.scopes.child(span)?;

    if needs.value() {
        let from = if let Some(from) = hir.from {
            expr(c, from, needs)?.apply(c)?;
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::Some,
                },
                from,
            );
            from
        } else {
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::None,
                },
                span,
            );
            span
        };

        c.scopes.alloc(from)?;

        let to = if let Some(to) = hir.to {
            expr(c, to, needs)?.apply(c)?;
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::Some,
                },
                to,
            );
            to
        } else {
            c.asm.push(
                Inst::Variant {
                    variant: InstVariant::None,
                },
                span,
            );
            span
        };

        c.scopes.alloc(to)?;

        let limits = match hir.limits {
            hir::ExprRangeLimits::HalfOpen => InstRangeLimits::HalfOpen,
            hir::ExprRangeLimits::Closed => InstRangeLimits::Closed,
        };

        c.asm.push(Inst::Range { limits }, span);
        c.scopes.free(span, 2)?;
    } else {
        if let Some(from) = hir.from {
            expr(c, from, needs)?.apply(c)?;
        }

        if let Some(to) = hir.to {
            expr(c, to, needs)?.apply(c)?;
        }
    }

    c.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a return expression.
#[instrument(span = span)]
fn expr_return<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: Option<&hir::Expr<'hir>>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm> {
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
        let clean = c.scopes.total(span)?;
        c.locals_pop(clean, span);
        c.asm.push(Inst::ReturnUnit, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a select expression.
#[instrument(span = span)]
fn expr_select<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprSelect<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    c.contexts.push(span.span());

    let len = hir.branches.len();
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
                    return Err(compile::Error::new(
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
        expr(c, branch.expr, Needs::Value)?.apply(c)?;
    }

    c.asm.push(Inst::Select { len }, span);

    for (branch, (label, _)) in branches.iter().enumerate() {
        c.asm.jump_if_branch(branch as i64, label, span);
    }

    if let Some((_, label)) = &default_branch {
        c.asm.push(Inst::Pop, span);
        c.asm.jump(label, span);
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    c.asm.jump(&end_label, span);

    for (label, branch) in branches {
        let span = branch.body;
        c.asm.label(&label)?;

        let expected = c.scopes.child(span)?;

        match branch.pat.kind {
            hir::PatKind::Path(&hir::PatPathKind::Ident(name, variable)) => {
                c.scopes.define(variable, name, branch.pat)?;
            }
            hir::PatKind::Ignore => {
                c.asm.push(Inst::Pop, span);
            }
            _ => {
                return Err(compile::Error::new(
                    branch.pat.span,
                    CompileErrorKind::UnsupportedSelectPattern,
                ));
            }
        }

        // Set up a new scope with the binding.
        expr(c, branch.body, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;
        c.asm.jump(&end_label, span);
    }

    if let Some((branch, label)) = default_branch {
        c.asm.label(&label)?;
        expr(c, branch, needs)?.apply(c)?;
    }

    c.asm.label(&end_label)?;

    c.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(span)?;

    Ok(Asm::top(span))
}

/// Assemble a try expression.
#[instrument(span = span)]
fn expr_try<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Expr<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let clean = c.scopes.total(span)?;
    let address = expr(c, hir, Needs::Value)?.apply_targeted(c)?;

    c.asm.push(
        Inst::Try {
            address,
            clean,
            preserve: needs.value(),
        },
        span,
    );

    if let InstAddress::Top = address {
        c.scopes.free(span, 1)?;
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
#[instrument(span = span)]
fn expr_tuple<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprSeq<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    macro_rules! tuple {
        ($variant:ident, $($var:ident),*) => {{
            let guard = c.scopes.child(span)?;

            let mut it = hir.items.iter();

            $(
            let $var = it.next().ok_or_else(|| compile::Error::msg(span, "items ended unexpectedly"))?;
            let $var = expr(c, $var, Needs::Value)?.apply_targeted(c)?;
            )*

            c.asm.push(
                Inst::$variant {
                    args: [$($var,)*],
                },
                span,
            );

            c.scopes.pop(guard, span)?;
        }};
    }

    if hir.items.is_empty() {
        c.asm.push(Inst::unit(), span);
    } else {
        match hir.items.len() {
            1 => tuple!(Tuple1, e1),
            2 => tuple!(Tuple2, e1, e2),
            3 => tuple!(Tuple3, e1, e2, e3),
            4 => tuple!(Tuple4, e1, e2, e3, e4),
            _ => {
                for e in hir.items {
                    expr(c, e, Needs::Value)?.apply(c)?;
                    c.scopes.alloc(e)?;
                }

                c.asm.push(
                    Inst::Tuple {
                        count: hir.items.len(),
                    },
                    span,
                );

                c.scopes.free(span, hir.items.len())?;
            }
        }
    }

    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a unary expression.
#[instrument(span = span)]
fn expr_unary<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprUnary<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    expr(c, hir.expr, Needs::Value)?.apply(c)?;

    match hir.op {
        ast::UnOp::Not(..) => {
            c.asm.push(Inst::Not, span);
        }
        ast::UnOp::Neg(..) => {
            c.asm.push(Inst::Neg, span);
        }
        op => {
            return Err(compile::Error::new(
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
#[instrument(span = span)]
fn expr_vec<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprSeq<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let count = hir.items.len();

    for e in hir.items {
        expr(c, e, Needs::Value)?.apply(c)?;
        c.scopes.alloc(e)?;
    }

    c.asm.push(Inst::Vec { count }, span);
    c.scopes.free(span, hir.items.len())?;

    // Evaluate the expressions one by one, then pop them to cause any
    // side effects (without creating an object).
    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a while loop.
#[instrument(span = span)]
fn expr_loop<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::ExprLoop<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    let continue_label = c.asm.new_label("while_continue");
    let then_label = c.asm.new_label("while_then");
    let end_label = c.asm.new_label("while_end");
    let break_label = c.asm.new_label("while_break");

    let var_count = c.scopes.total(span)?;

    let _guard = c.loops.push(Loop {
        label: hir.label.copied(),
        continue_label: continue_label.clone(),
        continue_var_count: var_count,
        break_label: break_label.clone(),
        break_var_count: var_count,
        needs,
        drop: None,
    });

    c.asm.label(&continue_label)?;

    let expected = if let Some(hir) = hir.condition {
        let then_scope = condition(c, hir, &then_label)?;
        let expected = c.scopes.push(then_scope);

        c.asm.jump(&end_label, span);
        c.asm.label(&then_label)?;
        Some(expected)
    } else {
        None
    };

    block(c, hir.body, Needs::None)?.apply(c)?;

    if let Some(expected) = expected {
        c.clean_last_scope(span, expected, Needs::None)?;
    }

    c.asm.jump(&continue_label, span);
    c.asm.label(&end_label)?;

    if needs.value() {
        c.asm.push(Inst::unit(), span);
    }

    // NB: breaks produce their own value / perform their own cleanup.
    c.asm.label(&break_label)?;
    Ok(Asm::top(span))
}

/// Assemble a `yield` expression.
#[instrument(span = span)]
fn expr_yield<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: Option<&hir::Expr<'hir>>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    if let Some(e) = hir {
        expr(c, e, Needs::Value)?.apply(c)?;
        c.asm.push(Inst::Yield, span);
    } else {
        c.asm.push(Inst::YieldUnit, span);
    }

    if !needs.value() {
        c.asm.push(Inst::Pop, span);
    }

    Ok(Asm::top(span))
}

/// Assemble a literal value.
#[instrument(span = span)]
fn lit<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: hir::Lit<'_>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm> {
    // Elide the entire literal if it's not needed.
    if !needs.value() {
        c.q.diagnostics.not_used(c.source_id, span, c.context());
        return Ok(Asm::top(span));
    }

    match hir {
        hir::Lit::Bool(boolean) => {
            c.asm.push(Inst::bool(boolean), span);
        }
        hir::Lit::Byte(byte) => {
            c.asm.push(Inst::byte(byte), span);
        }
        hir::Lit::Char(char) => {
            c.asm.push(Inst::char(char), span);
        }
        hir::Lit::Integer(integer) => {
            c.asm.push(Inst::integer(integer), span);
        }
        hir::Lit::Float(float) => {
            c.asm.push(Inst::float(float), span);
        }
        hir::Lit::Str(string) => {
            let slot = c.q.unit.new_static_string(span, string)?;
            c.asm.push(Inst::String { slot }, span);
        }
        hir::Lit::ByteStr(bytes) => {
            let slot = c.q.unit.new_static_bytes(span, bytes)?;
            c.asm.push(Inst::Bytes { slot }, span);
        }
    };

    Ok(Asm::top(span))
}

/// Assemble a local expression.
#[instrument(span = hir)]
fn local<'hir>(
    c: &mut Assembler<'_, 'hir>,
    hir: &hir::Local<'hir>,
    needs: Needs,
) -> compile::Result<Asm> {
    let load = |c: &mut Assembler<'_, 'hir>, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(c, hir.expr, needs)?.apply(c)?;
        Ok(())
    };

    let false_label = c.asm.new_label("let_panic");

    if pat(c, hir.pat, &false_label, &load)? {
        c.q.diagnostics
            .let_pattern_might_panic(c.source_id, hir, c.context());

        let ok_label = c.asm.new_label("let_ok");
        c.asm.jump(&ok_label, hir);
        c.asm.label(&false_label)?;
        c.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        );

        c.asm.label(&ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        c.asm.push(Inst::unit(), hir);
    }

    Ok(Asm::top(hir))
}

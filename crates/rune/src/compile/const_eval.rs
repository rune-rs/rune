//! Constant evaluation.
//!
//! Constants are not interpreted. They are compiled - by the same assembler
//! which compiles everything else - into an interior [`UnitBuilder`] which is
//! then snapshot into a [`Unit`] and executed by a [`Vm`].
//!
//! The interior unit is kept for the duration of the compilation. Constant
//! functions are assembled into it once and called as ordinary functions, so a
//! constant function which is called in a loop is compiled once rather than
//! re-interpreted per call, and recursion between constant functions costs
//! virtual machine call frames rather than compiler stack frames.

use core::mem::take;

use rust_alloc::rc::Rc;

use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};
use crate::ast::{self, Kind, Spanned};
use crate::compile::v2;
use crate::compile::{self, ErrorKind, ItemId, Location, UnitBuilder, WithSpan};
use crate::hir;
use crate::indexing::{index2, Indexer, Items, Scopes};
use crate::macros::{MacroContext, ToTokens, TokenStream};
use crate::query::Query;
use crate::runtime::unit::DefaultStorage;
use crate::runtime::{budget, RuntimeContext, Value, Vm, VmError, VmErrorKind, VmHaltInfo};
use crate::sync::Arc;
use crate::Hash;

/// The interior unit constants are compiled into before being evaluated.
#[derive(Default)]
pub(crate) struct ConstUnit {
    /// The unit being built.
    unit: UnitBuilder,
    /// Instruction storage associated with `unit`.
    storage: DefaultStorage,
    /// Counter used to hand out a distinct hash to every entry point.
    entry: u64,
    /// The runtime context to execute against.
    ///
    /// Building it copies the whole context, so it is only built if a constant
    /// is actually evaluated, and then only once.
    runtime: Option<Arc<RuntimeContext>>,
}

impl ConstUnit {
    /// Mirror metadata into the interior unit.
    ///
    /// Runtime type information and the constants associated with an item are
    /// registered with a unit as the item is first queried for, so the interior
    /// unit has to see the same registrations as the unit being compiled or it
    /// would not be able to construct or match the types a constant mentions.
    pub(crate) fn insert_meta(
        &mut self,
        span: &dyn Spanned,
        meta: &compile::meta::Meta,
        pool: &compile::Pool,
        query: &mut crate::query::QueryInner,
    ) -> compile::Result<()> {
        self.unit.insert_meta(span, meta, pool, query, false)
    }

    /// Allocate the hash of the next entry point.
    fn next_entry(&mut self) -> Hash {
        self.entry += 1;
        Hash::new(self.entry)
    }
}

/// What to assemble and call for a single evaluation.
pub(crate) enum Entry<'hir> {
    /// Evaluate an expression, which is assembled as the body of an entry
    /// point taking no arguments.
    Expr(&'hir hir::Exprs<'hir>, &'hir hir::Expr<'hir>),
    /// Evaluate a block, which is assembled as the body of an entry point
    /// taking no arguments.
    Block(&'hir hir::Exprs<'hir>, &'hir hir::Block<'hir>),
    /// Call the constant function with the given item, which becomes the entry
    /// point itself.
    Call(ItemId),
}

/// The body of an entry point.
enum Body<'hir> {
    Expr(&'hir hir::Expr<'hir>),
    Block(&'hir hir::Block<'hir>),
    Fn(&'hir hir::ItemFn<'hir>),
}

impl<'hir> Body<'hir> {
    /// The span the body is assembled at, used as the outermost warning
    /// context.
    fn span(&self) -> &'hir dyn Spanned {
        match *self {
            Body::Expr(hir) => hir,
            Body::Block(hir) => hir,
            Body::Fn(hir) => hir,
        }
    }
}

/// Evaluate an expression from within a macro, as `cx.eval(..)` does.
///
/// The expression arrives as an [`ast::Expr`], which a macro has parsed out of
/// its own input, so it has been through neither the grammar nor indexing. It
/// is emitted back into a token stream and re-parsed here, which is what
/// [`crate::worker`] does with the output of a macro, so that everything
/// downstream of it walks a tree over an explicit stack rather than recursing
/// over the syntax tree.
pub(crate) fn eval_ast(
    cx: &mut MacroContext<'_, '_, '_>,
    ast: &ast::Expr,
) -> compile::Result<Value> {
    let location = cx.item_meta.location;

    let mut stream = TokenStream::new();
    ast.to_tokens(cx, &mut stream)?;

    let tree = crate::grammar::token_stream(&stream)
        .max_nesting(cx.idx.q.options.max_depth)
        .root()?;

    let tree = Rc::new(tree);

    // The expression may contain items - a closure, or a block with items in
    // it - which have to be indexed before they can be lowered.
    {
        let items = Items::new(cx.idx.q.pool.item(cx.idx.item.id))?;

        let mut idx = Indexer {
            q: cx.idx.q.borrow(),
            source_id: cx.idx.source_id,
            items,
            scopes: Scopes::new()?,
            item: cx.idx.item,
            nested_item: cx.idx.nested_item,
            macro_depth: cx.idx.macro_depth,
            root: cx.idx.root,
            queue: cx.idx.queue.as_deref_mut(),
            loaded: cx.idx.loaded.as_deref_mut(),
            tree: &tree,
        };

        tree.parse_all(|p| index2::any(&mut idx, p))?;
    }

    let node = {
        let Some([root]) = tree.nodes() else {
            return Err(compile::Error::msg(
                ast,
                "expected single root in evaluated expression",
            ));
        };

        if !matches!(root.kind(), Kind::Root) {
            return Err(compile::Error::expected(root, Kind::Root));
        }

        let Some([node]) = root.nodes() else {
            return Err(compile::Error::msg(
                ast,
                "expected single evaluated expression",
            ));
        };

        if !matches!(node.kind(), Kind::Expr) {
            return Err(compile::Error::expected(node, Kind::Expr));
        }

        node
    };

    let arena = hir::Arena::new();

    let (hir, exprs) = {
        let mut hir_cx = hir::Ctxt::with_const(&arena, cx.idx.q.borrow(), location.source_id)?;
        let hir = node.parse(|p| hir::lowering2::expr(&mut hir_cx, p))?;
        (hir, hir_cx.take_exprs())
    };

    let mut q = cx.idx.q.borrow();
    eval(
        &mut q,
        location,
        &hir,
        Entry::Expr(&exprs, &hir),
        Vec::new(),
    )
}

/// Evaluate a constant, returning the value it produced.
pub(crate) fn eval<'hir>(
    q: &mut Query<'_, '_>,
    location: Location,
    span: &dyn Spanned,
    entry: Entry<'hir>,
    args: Vec<Value>,
) -> compile::Result<Value> {
    // The interior unit is moved out for the duration of the evaluation, so
    // that the query engine can be reborrowed against it while it is being
    // assembled into.
    //
    // This relies on an evaluation never starting another one. Metadata is
    // resolved while lowering, not while assembling, so nothing reachable from
    // here queries for an item - and queries are what evaluate constants.
    let mut cu = take(&mut q.inner.const_unit);
    let result = eval_with(q, &mut cu, location, span, entry, args);
    q.inner.const_unit = cu;
    result
}

fn eval_with<'hir>(
    q: &mut Query<'_, '_>,
    cu: &mut ConstUnit,
    location: Location,
    span: &dyn Spanned,
    entry: Entry<'hir>,
    args: Vec<Value>,
) -> compile::Result<Value> {
    let hash = match entry {
        Entry::Call(id) => q.const_fn_hash(id).with_span(span)?,
        Entry::Expr(exprs, hir) => {
            let hash = cu.next_entry();
            assemble(q, cu, location, hash, exprs, Body::Expr(hir))?;
            hash
        }
        Entry::Block(exprs, hir) => {
            let hash = cu.next_entry();
            assemble(q, cu, location, hash, exprs, Body::Block(hir))?;
            hash
        }
    };

    // Assembling the entry point discovers the constant functions it calls, and
    // assembling those discovers the ones they call in turn, so this drains
    // rather than iterates.
    while let Some(id) = q.inner.const_pending.pop() {
        let result = assemble_const_fn(q, cu, span, id);

        if result.is_err() {
            // The function is not in the unit, so let a later evaluation queue
            // it again and fail the same way, rather than calling a function
            // which was never assembled.
            q.inner.const_queued.remove(&id);
        }

        result?;
    }

    run(q, cu, span, hash, args)
}

/// Assemble a constant function into the interior unit.
fn assemble_const_fn(
    q: &mut Query<'_, '_>,
    cu: &mut ConstUnit,
    span: &dyn Spanned,
    id: ItemId,
) -> compile::Result<()> {
    let const_fn = q.const_fn_for(id).with_span(span)?;
    let hash = q.pool.item_type_hash(id);

    assemble(
        q,
        cu,
        const_fn.item_meta.location,
        hash,
        &const_fn.exprs,
        Body::Fn(&const_fn.hir),
    )
}

/// Assemble a body into the interior unit under the given hash.
fn assemble<'hir>(
    q: &mut Query<'_, '_>,
    cu: &mut ConstUnit,
    location: Location,
    hash: Hash,
    exprs: &'hir hir::Exprs<'hir>,
    body: Body<'hir>,
) -> compile::Result<()> {
    let options = q.options;
    let span = body.span();

    let ConstUnit { unit, storage, .. } = cu;

    let mut asm = unit.new_assembly(location);

    let (args, size) = {
        let mut q = q.with_unit(unit);
        let mut scopes = v2::Scopes::new(location.source_id)?;

        let mut cx = v2::Ctxt {
            source_id: location.source_id,
            q: q.borrow(),
            asm: &mut asm,
            exprs,
            scopes: &mut scopes,
            contexts: try_vec![span],
            breaks: v2::Breaks::new(),
            options,
            select_branches: Vec::new(),
            drop: Vec::new(),
            const_eval: true,
        };

        let args = match body {
            Body::Expr(hir) => {
                v2::assemble::const_expr(&mut cx, hir)?;
                0
            }
            Body::Block(hir) => {
                v2::assemble::const_block(&mut cx, hir)?;
                0
            }
            Body::Fn(hir) => {
                v2::assemble::fn_from_item_fn(&mut cx, hir)?;
                hir.args.len()
            }
        };

        (args, cx.scopes.size())
    };

    unit.new_const_function(location, hash, args, asm, storage, size)
}

/// Snapshot the interior unit and execute the given entry point in it.
fn run(
    q: &mut Query<'_, '_>,
    cu: &mut ConstUnit,
    span: &dyn Spanned,
    hash: Hash,
    args: Vec<Value>,
) -> compile::Result<Value> {
    let runtime = match &cu.runtime {
        Some(runtime) => runtime.clone(),
        None => {
            let runtime = Arc::try_new(q.context.runtime().with_span(span)?).with_span(span)?;
            cu.runtime = Some(runtime.clone());
            runtime
        }
    };

    let storage = cu.storage.try_clone().with_span(span)?;
    let unit = Arc::try_new(cu.unit.snapshot(storage).with_span(span)?).with_span(span)?;

    let mut vm = Vm::new(runtime, unit);
    let budget = q.options.const_budget;

    match budget::with(budget, || vm.call(hash, args)).call() {
        Ok(value) => Ok(value),
        // A constant which does not terminate halts the machine rather than
        // erroring, so it is reported as the budget being spent instead of as
        // an unexpected halt.
        Err(error) => match error.into_kind() {
            VmErrorKind::Halted {
                halt: VmHaltInfo::Limited,
            } => Err(compile::Error::new(
                span,
                ErrorKind::ConstBudgetExceeded { budget },
            )),
            kind => Err(compile::Error::new(
                span,
                ErrorKind::from(VmError::new(kind)),
            )),
        },
    }
}

impl Query<'_, '_> {
    /// Get the hash of the constant function with the given item, queueing it
    /// to be assembled into the interior unit if it has not been already.
    pub(crate) fn const_fn_hash(&mut self, id: ItemId) -> alloc::Result<Hash> {
        if self.inner.const_queued.try_insert(id)? {
            self.inner.const_pending.try_push(id)?;
        }

        Ok(self.pool.item_type_hash(id))
    }
}

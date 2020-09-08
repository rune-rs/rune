# Compiler guide

This is intended to be a guide into the compiler architecture for Rune for
people who want to hack on it.

> **Rune is heavily in development** and this is bound to change in the future.

Compiling a rune program involves the following stages:

* Parse and queue the initial source files into [`AST`], specified by
  [`Source::insert_default`].
* **Indexing and macro expansion**, which processes tasks in the [`Worker`]
  queue until it is empty. These are:
  * `Task::Index` - Index language items.
  * `Task::Import` - Process imports, expands `use` items and indexes all
    imported names.
  * `Task::ExpandMacro` - Expand macros (which can add more tasks to the worker
    queue).
* **Compilation** which processes a queue of items to be compiled.

[`AST`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/ast
[`Source::insert_default`]: https://docs.rs/runestick/0/runestick/struct.Source.html#method.insert_default
[`Worker`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/worker.rs

## Indexing

Indexing is primarily handled through the [`Index<T>`] trait, which are
implemented for the type being indexed on top of the [`Indexer`].

This walks through the [`AST`] to be indexed and construct [components] into an
item path for every:
* Functions, which adds components named after the function. `fn foo` would add
  `foo`.
* Closures, which adds `$closure<number>` components.
* Block, which adds `$block<number>` components.
* Async blocks, which adds `$async<number>` components.
* Macro calls, which adds `$macro<number>` components, and queues a
  `Task::ExpandMacro` task.

[`AST`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/ast
[components]: https://github.com/rune-rs/rune/blob/master/crates/runestick/src/item.rs#L138
[`Index<T>`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/index.rs
[`Indexer`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/index.rs

## Compilation

The compilation stage processed the entire [`AST`] of every function that is
queued to be compiled and generates a sequence of instructions for them through
implementations of the [`Compile<T>`] trait.

Each item in the AST is relatively isolated when they are being compiled. This
is one of the benefits of compiling for a stack-based virtual machine in that
the compilation stage is relatively simple.

While compiling we keep track of the following state in the [`Compiler`]

The source file and id that we are compiling for and global storage used for
macro-generated identifiers and literals. This is used to resolve values from
the AST through the corresponding [`Resolve<T>`] implementation.

Our local variable scopes using [`Scopes`]. Each block creates a new scope of
local variables, and this is simply a number that is incremented each time a
variable is allocated. These can either be named or anonymous. Each named
variable is associated with an offset relative to the current [call
frame](./call_frames.md) that can be looked up when a variable needs to be used.

A stack of [`Loops`] currently being processed and their corresponding state.
Because if we encounter a [`break` expression], any local state they create and
all local variables up until the break needs to be cleaned up.

[The `item` path](./items_imports.md) which is being traversed through
[`Items`]. This is populated in the same way as during indexing. If we encounter
an item which has been indexed like a [`closure` expression]. This actually has
two separate `Compile<T>` implementations.

* `Compile<(ast::ExprClosure, &[CompileMetaCapture])>` - which is used when
  compiling the closure function.
* `Compile<(&ast::ExprClosure, Needs)>` - which is for compiling the closure
  invocation.

The function is compiled and inserted into the unit separately at a given item
(like `foo::$block0::closure1`). And when we invoke the closure, we *call* this
item.

[`AST`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/ast
[`Compile<T>`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/compile
[`Compiler`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compiler.rs
[`Resolve<T>`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/traits.rs
[`Scopes`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/scopes.rs
[`Loops`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/loops.rs
[`break` expression]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compile/expr_break.rs
[`Items`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/items.rs
[`closure` expression]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compile/expr_closure.rs
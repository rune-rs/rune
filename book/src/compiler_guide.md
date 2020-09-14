# Compiler guide

This is intended to be a guide into the compiler architecture for Rune for
people who want to hack on it.

> **Rune is heavily in development** and this is bound to change in the future.

Compiling a rune program involves the following stages:

* Parse and queue the initial source files into [`AST`], specified by
  [`Source::insert`].
* **Indexing and macro expansion**, which processes tasks in the [`Worker`]
  queue until it is empty. These are:
  * `Task::Index` - Index language items.
  * `Task::Import` - Process imports, expands `use` items and indexes all
    imported names.
  * `Task::ExpandMacro` - Expand macros (which can add more tasks to the worker
    queue).
* **Compilation** which processes a queue of items to be compiled.

[`AST`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/ast
[`Source::insert`]: https://docs.rs/runestick/0/runestick/struct.Source.html#method.insert
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

This stage uses the [`Query`] system to look up metadata about external items,
and any external item queried for is subsequently queued up to be built.

Consider the following unit:

```rune
{{#include ../../scripts/book/compiler_guide/dead_code.rn}}
```

Let's dump all dynamic functions in it:

```text
$> cargo run -- scripts/book/compiler_guide/dead_code.rn --dump-functions
# dynamic functions
0xe7fc1d6083100dcd = main()
0x20c6d8dd92b51018 = main::$block0::foo()
---
== 2 (59.8µs)
```

As you can see, the code for `main::$block0::bar` was *never generated*. This is
because it's a local function that is never called. And therefore never queried
for. So it's never queued to be built in the compilation stage.

## State during compilation

Each item in the AST is relatively isolated while they are being compiled. This
is one of the benefits of compiling for a stack-based virtual machine - the
compilation stage is relatively simple and *most* reasoning about what
instructions to emit can be made locally.

> Note that this quickly changes if you want to perform most forms of
> optimizations. But it's definitely true for naive (and therefore fast!) code
> generation.

While compiling we keep track of the following state in the [`Compiler`]

The source file and id that we are compiling for and global storage used for
macro-generated identifiers and literals. This is used to resolve values from
the AST through the corresponding [`Resolve`] implementation. An example of this
is the [`Resolve` implementation of `LitStr`].

We keep track of local variables using [`Scopes`]. Each block creates a new
scope of local variables, and this is simply a number that is incremented each
time a variable is allocated. These can either be named or anonymous. Each named
variable is associated with an offset relative to the current [call
frame](./call_frames.md) that can be looked up when a variable needs to be used.

We maintain information on loops we're through [`Loops`]. This is a stack that
contains every loop we are nested in, information on the label in which the loop
terminates, and locals that would have to be cleaned up in case we encounter a
[`break` expression].

We maintain the current [item path](./items_imports.md) which is being traversed
through [`Items`]. This is populated in the same way as during indexing. We use
this to query for compile meta or other things which were indexed, like the AST
produced by expanded macros. Another example is if we encounter a [`closure`
expression] in this phase we generate an instruction to create a closure rather
than constructing its body. This is actually accomplished through two separate
`Compile<T>` implementations.

* `Compile<(ast::ExprClosure, &[CompileMetaCapture])>` - which is used when
  compiling the closure function from the compile queue.
* `Compile<(&ast::ExprClosure, Needs)>` - which is for compiling the closure
  invocation.

The closure procedure would be compiled and inserted into the unit separately at
a given item (like `foo::$block0::closure1`). And when we invoke the closure, we
*call* this item.

We can see this call by dumping all the dynamic functions in the following
script:

```rune
{{#include ../../scripts/book/compiler_guide/closures.rn}}
```

```text
$> cargo run -- scripts/book/compiler_guide/closures.rn --dump-instructions --dump-functions
# instructions
fn main() (0xe7fc1d6083100dcd):
  0000 = fn 0x9aa62663879132fb // closure `main::$block0::$closure0`
  0001 = copy 0 // var `callable`
  0002 = call-fn 0
  0003 = pop
  0004 = pop
  0005 = return-unit

fn main::$block0::$closure0() (0x9aa62663879132fb):
  0006 = integer 42
  0007 = return
# dynamic functions
0x9aa62663879132fb = main::$block0::$closure0()
0xe7fc1d6083100dcd = main()
== () (108.6µs)
```

A function pointer is pushed on the stack `fn 0x9aa62663879132fb`, then copied
and called with zero arguments.

[`AST`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/ast
[`Compile<T>`]: https://github.com/rune-rs/rune/tree/master/crates/rune/src/compile
[`Query`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/query.rs
[`Compiler`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compiler.rs
[`Resolve`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/traits.rs
[`Resolve` implementation of `LitStr`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/ast/lit_str.rs
[`Scopes`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/scopes.rs
[`Loops`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/loops.rs
[`break` expression]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compile/expr_break.rs
[`Items`]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/items.rs
[`closure` expression]: https://github.com/rune-rs/rune/blob/master/crates/rune/src/compile/expr_closure.rs
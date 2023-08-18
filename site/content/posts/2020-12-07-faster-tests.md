+++
title = "Faster integration tests"
date = 2020-12-07
draft = false
template = "post.html"

[taxonomies]
categories = ["rust"]
tags = ["tips", "rust"]

[extra]
author = "John-John Tedro"
+++

This is just a quick post to outline a trick that can be used to speed up
building of projects which has a lot of integration tests.

<!-- more -->

The standard way to run and build integration tests in Rust is to make use of a
cargo feature called [`autotests`]. This will scan your project for a directory
called `tests` and build an integration test for every file found.

So what happens if you have a lot of tests?

[`autotests`]: https://doc.rust-lang.org/cargo/reference/cargo-targets.html#target-auto-discovery

This is the [`tests` directory used by Rune](https://github.com/rune-rs/rune/tree/main/tests/tests):

```text
tests/bugfixes.rs
tests/collections.rs
tests/compiler_attributes.rs
tests/compiler_expr_assign.rs
tests/compiler_expr_binary.rs
tests/compiler_fn.rs
tests/compiler_general.rs
tests/compiler_literals.rs
tests/compiler_paths.rs
tests/compiler_use.rs
tests/compiler_visibility.rs
tests/compiler_warnings.rs
tests/core_macros.rs
tests/destructuring.rs
tests/external_ops.rs
tests/for_loop.rs
tests/getter_setter.rs
tests/iterator.rs
tests/moved.rs
tests/reference_error.rs
tests/stmt_reordering.rs
tests/test_continue.rs
tests/test_iter.rs
tests/test_option.rs
tests/test_quote.rs
tests/test_range.rs
tests/test_result.rs
tests/type_name_native.rs
tests/type_name_rune.rs
tests/vm_arithmetic.rs
tests/vm_assign_exprs.rs
tests/vm_async_block.rs
tests/vm_blocks.rs
tests/vm_closures.rs
tests/vm_const_exprs.rs
tests/vm_early_termination.rs
tests/vm_function.rs
tests/vm_general.rs
tests/vm_generators.rs
tests/vm_is.rs
tests/vm_lazy_and_or.rs
tests/vm_literals.rs
tests/vm_match.rs
tests/vm_not_used.rs
tests/vm_option.rs
tests/vm_pat.rs
tests/vm_result.rs
tests/vm_streams.rs
tests/vm_test_external_fn_ptr.rs
tests/vm_test_from_value_derive.rs
tests/vm_test_imports.rs
tests/vm_test_instance_fns.rs
tests/vm_test_linked_list.rs
tests/vm_test_mod.rs
tests/vm_try.rs
tests/vm_tuples.rs
tests/vm_typed_tuple.rs
tests/vm_types.rs
tests/wildcard_imports.rs
```

All in all this is *59 test binaries* that need to be built and executed.
Linking binaries can be a painfully slow process. And especially when this has
to happen every time a dependency is changed. Something we expect to happen
quite *frequently* in the projects being developed!

So if we let `autotests` do its thing, let's see how long it takes to run our
tests:

```sh
# make sure dependencies are built
$> cargo clean && cargo build
$> time cargo test
cargo test  451,83s user 59,81s system 1170% cpu 43,697 total
```

Assuming this doesn't cause issues for you. A faster way to do this would be to
build a single binary containing all tests.

Next I'm gonna showcase a hack you can do to make this happen with minor
modifications to your project.

First we need to disable `autotests` in `Cargo.toml` and specify an entrypoint
to our one integration test:

```toml
[package]
name = "rune-tests"
# **snip**

# disable autodiscovery of tests
autotests = false

# add our entrypoint
[[test]]
name = "test"
path = "test.rs"
```

Next is where the magic happens. We write a `build.rs` extension to perform our
own autodiscovery:

```rust
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::Path;

fn discover_tests() -> io::Result<()> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .expect("missing CARGO_MANIFEST_DIR");

    let out_dir = env::var_os("OUT_DIR")
        .expect("missing OUT_DIR");

    let mut f = fs::File::create(Path::new(&out_dir).join("tests.rs"))?;

    let tests = Path::new(&manifest_dir).join("tests");

    for entry in fs::read_dir(tests)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() || path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        if let Some(stem) = path.file_stem() {
            let path = path.canonicalize()?;

            writeln!(f, "#[path = {:?}]", path.display())?;
            writeln!(f, "mod {};", stem.to_string_lossy())?;
        }
    }

    Ok(())
}

fn main() {
    discover_tests().expect("Failed to discover tests");
}
```

This writes a generated file in `OUT_DIR` called `tests.rs` which contains the
necessary module enumeration of our test files so that Rust can build it as a
single project.

> Note the use of `#[path = ..]` above which uses an absolute path. This is
> necessary because it would otherwise look for the modules in `OUT_DIR` instead
> of the correct file in our `tests` directory. There are other ways to do this,
> but this provides us with the modules as part of the test names when we run
> them.

Finally we include the generated file in `test.rs` which we defined as our
entrypoint in `Cargo.toml`:

```rust
include!(concat!(env!("OUT_DIR"), "/tests.rs"));
```

Now if we do the same dance again we'll notice a significant speedup:

```sh
$> cargo clean && cargo build
$> time cargo test
cargo test  33,86s user 3,73s system 315% cpu 11,912 total
```

The entirety of the speedup lies within building and running a single binary
instead of *59 separate* ones. This won't work for everyone, and obviously only
speeds things up significantly if you happen to have a lot of integration tests.

But at least it's good to be aware of what's going on, and that there's a fairly
easy way to speed things up if needed.

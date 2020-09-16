# Instance functions

Instance functions are functions that are associated to a specific type of
variable. When called they take the form `value.foo()`, where the *instance*
is the first part `value`. And the *instance function* is `foo()`.

These are a bit special in Rune. Since Rune is a dynamic programming language we
can't tell at compile time which instance any specific `value` can be. So
instance functions must be looked up at runtime.

```rune
{{#include ../../scripts/book/instance_functions/missing_instance_fn.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/instance_functions/missing_instance_fn.rn
error: virtual machine error
   ┌─ scripts/book/instance_functions/missing_instance_fn.rn:11:5
   │
11 │     foo.bar();
   │     ^^^^^^^^^ missing instance function `0xfb67fa086988a22d` for `type(0xc153807c3ddc98d7)``
```

> Note: The error is currently a bit nondescript. But in the future we will be
> able to provide better diagnostics by adding debug information.

What you're seeing above are type and function hashes. These uniquely identify
the item in the virtual machine and is the result of a deterministic computation
based on its item. So the hash for the item `Foo::new` will always be the same.

In Rust, we can calculate this hash using the `Hash::type_hash` method:

```rune
{{#include ../../crates/rune/examples/function_hash.rs}}
```

```text
$> cargo run --example function_hash
0xb5dc92ab43cb37d9
0xb5dc92ab43cb37d9
```

The exact implementation of the hash function is currently not defined, but will
be stabilized and documented in a future release.

## Defining instance functions in Rust

Native instance functions are added to a runtime environment using the
[`Module::inst_fn`] and [`Module::async_inst_fn`] functions. The type is
identified as the first argument of the instance function, and must be a type
registered in the module using [`Module::ty`].

```rust,noplaypen
{{#include ../../crates/rune/examples/custom_instance_fn.rs}}
```

```text
$> cargo run --example custom_instance_fn
output: 11
```

For more examples on how modules can be used you can have a look at the source
for the [`rune-modules`] crate.

[`Module::inst_fn`]: https://docs.rs/runestick/0.5.3/runestick/struct.Module.html#method.inst_fn
[`Module::async_inst_fn`]: https://docs.rs/runestick/0.5.3/runestick/struct.Module.html#method.async_inst_fn
[`Module::ty`]: https://docs.rs/runestick/0.5.3/runestick/struct.Module.html#method.ty
[`rune-modules`]: https://github.com/rune-rs/rune/tree/master/crates/rune-modules

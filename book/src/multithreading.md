# Multithreading

Rune is thread safe, but the [`Vm`] does not implement `Sync` so cannot directly
be shared across threads. This section details instead how you are intended to
use Rune in a multithreaded environment.

Compiling a [`Unit`] and a [`RuntimeContext`] are expensive operations compared
to the cost of calling a function. So you should try to do this as little as
possible. It is appropriate to recompile a script when the source of the script
changes. See the [Hot reloading] section for more information on this.

Once you have a `Unit` and a `RuntimeContext` they are thread safe and can be
used by multiple threads simultaneously through `Arc<Unit>` and
`Arc<RuntimeContext>`. Constructing a `Vm` with these through `Vm::new` is a
very cheap operation.

```rust
let unit: Arc<Unit> = /* todo */;
let context: Arc<RuntimeContext> = /* todo */;

std::thread::spawn(move || {
    let mut vm = Vm::new(unit, context);
    let value = vm.call(["function"], (42,))?;
    Ok(())
});
```

> Virtual machines do allocate memory. To avoide this overhead you'd have to
> employ more advanced techniques, such as storing virtual machines in a pool or
> [thread locals]. Once a machine has been acquired the `Unit` and
> `RuntimeContext` associated with it can be swapped out to the ones you need
> using [`Vm::unit_mut`] and [`Vm::context_mut`] respectively.

Using [`Vm::send_execute`] is a way to assert that a given execution is thread
safe. And allows you to use Rune in asynchronous multithreaded environments,
such as Tokio. This is achieved by ensuring that all captured arguments are
[`ConstValue`]'s, which in contrast to [`Value`]'s are guaranteed to be
thread-safe:

```rust
{{#include ../../examples/examples/tokio_spawn.rs}}
```

Finally [`Function::into_sync`] exists to coerce a function into a
[`SyncFunction`], which is a thread-safe variant of a regular [`Function`]. This
is a fallible operation since all values which are captured in the function-type
in case its a closure has to be coerced to [`ConstValue`]. If this is not the
case, the conversion will fail.

[`ConstValue`]: https://docs.rs/rune/latest/rune/runtime/enum.ConstValue.html
[`Function::into_sync`]: https://docs.rs/rune/latest/rune/runtime/struct.Function.html#method.into_sync
[`Function`]: https://docs.rs/rune/latest/rune/runtime/struct.Function.html
[`notify`]: https://docs.rs/notify
[`RuntimeContext`]: https://docs.rs/rune/latest/rune/runtime/struct.RuntimeContext.html
[`SyncFunction`]: https://docs.rs/rune/latest/rune/runtime/struct.SyncFunction.html
[`Unit`]: https://docs.rs/rune/latest/rune/runtime/struct.Unit.html
[`Value`]: https://docs.rs/rune/latest/rune/runtime/enum.Value.html
[`Vm::context_mut`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html#method.context_mut
[`Vm::send_execute`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html#method.send_execute
[`Vm::unit_mut`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html#method.unit_mut
[`Vm`]: https://docs.rs/rune/latest/rune/runtime/struct.Vm.html
[Hot reloading]: ./hot_reloading.md
[thread locals]: https://doc.rust-lang.org/std/macro.thread_local.html

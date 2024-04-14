# Hot reloading

Compiling a [`Unit`] and a [`RuntimeContext`] are expensive operations compared
to the cost of calling a function. So you should try to do this as little as
possible. It is appropriate to recompile a script when the source of the script
changes. This section provides you with details for how this can be done when
loading scripts from the filesystem.

A typical way to accomplish this is to watch a scripts directory using the
[`notify` crate]. This allow the application to generate events whenever changes
to the directory are detected. See the [`hot_reloading` example] and in
particular the [`PathReloader`] type.

```rust
{{#include ../../examples/examples/hot_reloading.rs}}
```

[`notify` crate]: https://docs.rs/notify
[`Unit`]: https://docs.rs/rune/latest/rune/runtime/unit/struct.Unit.html
[`hot_reloading` example]: https://github.com/rune-rs/rune/blob/main/examples/examples/hot_reloading.rs
[`PathReloader`]: https://github.com/rune-rs/rune/blob/main/examples/examples/hot_reloading/path_reloader.rs


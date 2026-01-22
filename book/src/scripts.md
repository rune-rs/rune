Rune can be run in a special mode called a "script" mode. This is the simplest
way of using Rune.

This is intended to support what looks like inline execution of code, and is
implemented to allow for evaluating expressions like `value + 40`.

In order to run a program in script mode [`Options::script`] has to be set,
any implicit arguments passed into the script must then be specified through
[`Prepare::with_args`].

```rust,noplaypen
{{#include ../../examples/examples/scripts.rs}}
```

Behind the scenes script mode defines an unnamed function which can be addressed
as `Hash::EMPTY`. The arguments that has to be passed into this function is
defined through [`Prepare::with_args`]. These variables are *not* global
variables. They are not addressible from other functions.

[`Options::script`]: https://docs.rs/rune/latest/rune/struct.Options.html#method.script
[`Prepare::with_args`]: https://docs.rs/rune/latest/rune/struct.Build.html#method.with_args

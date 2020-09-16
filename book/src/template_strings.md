# Template strings

If you've been paying attention on previous sections you might have seen odd
looking strings like `` `Hello {name}` ``. These are called *template strings*,
and allow you to conveniently build strings using variables from the
environment.

```rune
{{#include ../../scripts/book/template_strings/basic_template.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/template_strings/basic_template.rn
I am 30 years old!
== () (4.5678ms)
```

Template strings are accelerated by the Vm, each argument uses a *display
protocol* and it can be very efficient to build complex strings out of it.

## The `STRING_DISPLAY` protocol

The `STRING_DISPLAY` protocol is a function that can be implemented by any
*external* type which allows it to be used in a template string.

It expects a function with the signature `fn(&self, buf: &mut String) -> fmt::Result`.

```rust,noplaypen
use std::fmt::Write as _;
use std::fmt;

#[derive(Debug)]
pub struct StatusCode {
    inner: u32,
}

impl StatusCode {
    fn display(&self, buf: &mut String) -> fmt::Result {
        write!(buf, "{}", self.inner)
    }
}

pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["http"]);
    module.inst_fn(runestick::STRING_DISPLAY, StatusCode::display)?;
    Ok(module)
}
```

This is what allows status codes to be formatted into template strings, any
types which do not implement this protocol will fail to run.

```rune
{{#include ../../scripts/book/template_strings/not_a_template.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/template_strings/not_a_template.rn
error: virtual machine error
  ┌─ scripts/book/template_strings/not_a_template.rn:3:13
  │
3 │     println(`{vec}`);
  │             ^^^^^^^ `vector` does not implement the `string_display` protocol
```

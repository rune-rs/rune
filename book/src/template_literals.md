# Template literals

If you've been paying attention on previous sections you might have seen odd
looking strings like `` `Hello ${name}` ``. These are called *template
literals*, and allow you to conveniently build strings using variables from the
environment.

> Template literals are [a concept borrowed from EcmaScript].

```rune
{{#include ../../scripts/book/template_literals/basic_template.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/template_literals/basic_template.rn
"I am 30 years old!"
```

Template strings are accelerated by the Vm, each argument uses a *display
protocol* and it can be very efficient to build complex strings out of it.

[a concept borrowed from EcmaScript]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals

## The `STRING_DISPLAY` protocol

The `STRING_DISPLAY` protocol is a function that can be implemented by any
*external* type which allows it to be used in a template string.

It expects a function with the signature `fn(&self, buf: &mut String) -> fmt::Result`.

```rust,noplaypen
use rune::{ContextError, Module};
use rune::runtime::{Protocol, Formatter};
use std::fmt::Write as _;
use std::fmt;

#[derive(Debug)]
pub struct StatusCode {
    inner: u32,
}

impl StatusCode {
    #[rune::function(protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::new(["http"]);
    module.function_meta(StatusCode::string_display)?;
    Ok(module)
}
```

This is what allows status codes to be formatted into template strings, any
types which do not implement this protocol will fail to run.

```rune
{{#include ../../scripts/book/template_literals/not_a_template.rn}}
```

```text
$> cargo run --bin rune -- run scripts/book/template_literals/not_a_template.rn
== ! (`Vec` does not implement the `string_display` protocol (at 5)) (77.7µs)
error: virtual machine error
  ┌─ scripts/book/template_literals/not_a_template.rn:3:9
  │
3 │     dbg(`${vec}`);
  │         ^^^^^^^^ `Vec` does not implement the `string_display` protocol
```

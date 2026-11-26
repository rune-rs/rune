# Declaration sources

A host which installs native functions and types can describe them in a
*declaration source*: a file whose name ends in `.d.rn`. It is written in the
same syntax as any other Rune source, but holds only signatures, type
definitions and their documentation:

```rune
/// Say hello.
pub fn hello(value: u64) -> u64;

pub mod net {
    /// Fetch the body behind `url`.
    pub async fn fetch(url: String) -> String;
}

pub struct Point { x: f64, y: f64 }

impl Point {
    fn new(x: f64, y: f64) -> Point;
    fn len(self) -> f64;
}
```

Functions have no bodies, and unlike in an ordinary source arguments, return
values and fields may carry types. Anything which is not a declaration - a
function body, a constant, a statement, a macro call - is reported where it was
written.

## What the compiler does with it

Nothing in a declaration source is compiled into the unit. What it declares is
instead resolved the way an item the host installed natively is, so a script
compiled beside it may call `hello(1)`, `net::fetch(url)` and `Point::new(1.0,
2.0)` and name `Point` without the host being there. This is what lets a
language server or `rune check` work on scripts written for a host they do not
have.

- An item the native context already has wins over a declaration of the same
  name, and no conflict is reported, so a declaration source keeps working when
  the script is compiled against the real host.
- A function nobody declared is still a compile error.
- Calling a declared function which the host does not provide when the script
  runs fails at that call with an error naming the function, not at compile
  time.
- Declarations are public, since what they stand for are native items. The
  types written in them are documentation; the compiler does not check them.
- The variants of a declared enum can be constructed and matched on like those
  of a native enum. Constructing one without the host fails at run time,
  the way calling a declared function the host does not provide does.
- `use` is not allowed in a declaration source. Since types in it are only
  documentation there is nothing for an import to resolve. A type is looked up
  among the types the same source declares, then the prelude, then the native
  context.

## Documentation

`rune doc` documents what a declaration source declares: its functions, types,
methods and variants, with their documentation and the types written on their
arguments and return values.

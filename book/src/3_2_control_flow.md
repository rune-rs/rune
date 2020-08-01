# Control Flow

Rune supports your typical forms of control flow.

## `if` Expressions

If expressions are conditionals that can have one or more branches preceded by a
condition.

```rune
fn main() {
    let number = 3;

    if number < 5 {
        dbg("condition was true");
    } else {
        dbg("condition was false");
    }
}
```

## Loops

To repeat the execution of code Rune gives you `while`, `for` and `loop`.

#### Repeating with `loop`

```rune
fn main() {
    loop {
        dbg("forever");
    }
}
```

#### Iterating loops with `for`

```rune
use std::iter::range;

fn main() {
    for n in range(0, 10) {
        dbg(n);
    }
}
```

#### Conditional loops with `while`

```rune
use std::iter::range;

fn main() {
    let n = 0;

    while n < 10 {
        dbg(n);
        n = n + 1;
    }
}
```
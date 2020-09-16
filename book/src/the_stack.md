# The stack

Runestick is a stack-based virtual machine. It has two primary places where
things are stored. *The stack* and *the heap*. It has no registers.

Instructions in the virtual machine operate off the stack. Let's take a look at
the add operation with `--trace` and `--dump-stack`.

```rune
{{#include ../../scripts/book/the_stack/add.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/the_stack/add.rn --trace --dump-stack
fn main() (0xe7fc1d6083100dcd):
  0000 = integer 1
    0+0 = 1
  0001 = integer 3
    0+0 = 1
    0+1 = 3
  0002 = add
    0+0 = 4
  0003 = return
    *empty*
== 4 (7.7691ms)
# stack dump after halting
frame #0 (+0)
    *empty*
```

Let's examine the stack after each instruction.

```text
  0000 = integer 1
    0+0 = 1
```

We evaluate the `integer 1` instruction, which pushes an integer with the value
`1` onto the stack.

```text
  0001 = integer 3
    0+0 = 1
    0+1 = 3
```

We evaluate the `integer 3` instruction, which pushes an integer with the value
`3` onto the stack.

```text
  0002 = add
    0+0 = 4
```

We evaluate the `add` instruction which pops two values from the stack and adds
them together. Two integers in this instance would use built-in accelerated
implementations which performs addition.

```text
  0003 = return
== 4 (7.7691ms)
```

We `return` from the virtual machine. The last value of the stack will be popped
as the return value.

```text
# stack dump
frame #0 (+0)
```

This is the stack dump we see after the virtual machine has exited.
It tells us that at call frame `#0 (+0)`, the last and empty call frame at stack
position `+0` there is nothing on the stack.

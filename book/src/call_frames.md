# Call frames

Call frames are a cheap function isolation mechanism available in the virtual
machine. They create a subslice in the stack, preventing the vm from accessing
any values that are at an address below the current call frame.

```rust,noplaypen
{{#include ../../scripts/book/the_stack/call_and_add.rn}}
```

```text
$> cargo run -- scripts/book/the_stack/call_and_add.rn --trace --dump-stack
fn main() (0xe7fc1d6083100dcd):
  0005 = integer 3
    0+0 = 3
  0006 = integer 1
    0+0 = 3
    0+1 = 1
  0007 = integer 2
    0+0 = 3
    0+1 = 1
    0+2 = 2
  0008 = call 0xbfd58656ec9a8ebe, 2 // fn `foo`
=> frame 1 (1):
    1+0 = 1
    1+1 = 2
fn foo(arg, arg) (0xbfd58656ec9a8ebe):
  0000 = copy 0 // var `a`
    1+0 = 1
    1+1 = 2
    1+2 = 1
  0001 = copy 1 // var `b`
    1+0 = 1
    1+1 = 2
    1+2 = 1
    1+3 = 2
  0002 = add
    1+0 = 1
    1+1 = 2
    1+2 = 3
  0003 = clean 2
    1+0 = 3
  0004 = return
<= frame 0 (0):
    0+0 = 3
    0+1 = 3
  0009 = copy 0 // var `a`
    0+0 = 3
    0+1 = 3
    0+2 = 3
  0010 = add
    0+0 = 3
    0+1 = 6
  0011 = clean 1
    0+0 = 6
  0012 = return
    *empty*
== 6 (45.8613ms)
# full stack dump after halting
  frame #0 (+0)
    *empty*
```

We're not going to go through each instruction step-by-step as in the last
section. Instead I will point out the things which are worth noting.

We have an instruction shown as `call 0xbfd58656ec9a8ebe, 2`, which means tells
the virtual machine to call the function with the hash `0xbfd58656ec9a8ebe`, and
use the top two values on the stack as arguments to this function.

We can see that the first argument `a` is on the *lowest* position, and the
second argument `b` is on the *highest* position. Let's examine this function call closer.


```text
    0+0 = 3
    0+1 = 1
    0+2 = 2
  0008 = call 0xbfd58656ec9a8ebe, 2 // fn `foo`
=> frame 1 (1):
    1+0 = 1
    1+1 = 2
```

Here we can see the call being executed. A new stack frame `frame 1` is
allocated, and we can see that it contains two items, `1` and `2`.

We can also see that the items are offset from position `1`. `1+0` and `1+1`.
This is to indicate that the call frame relative position of the items are `0`
and `1`, but the global stack location is `1+0`, which is `1`. And `1+1` which
is `2`. The value `3` at `0+0` is no longer visible to the call frame. But we
can see it become visible again later one when the call frame returns.

```
    1+0 = 1
    1+1 = 2
    1+2 = 3
  0003 = clean 2
    1+0 = 3
  0004 = return
<= frame 0 (0):
    0+0 = 3
    0+1 = 3
```

Here we can see the `clean 2` instruction. Which tells the vm to preserve the
top of the stack `1+2`, and clean two items off it. Then we `return`, after
which we can see that we return to `frame 0`, which now has `0+0` visible *and*
our return value at `0+1`.

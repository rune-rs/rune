# Call frames

Call frames are a cheap isolation mechanism available in the virtual machine.
They define a subslice in the stack, preventing the vm from accessing values
that are outside of the slice.

They have the following rules:
* Instructions cannot access values outside of their current call frame.
* When we return from the call frame the subslice must be empty.

If any these two conditions aren't maintained, the virtual machine will error.

Call frames fill two purposes. The subslice provides a well-defined variable
region. Stack-relative operations like `copy 0` are always defined relative to
the top of their call frame. Where `copy 0` would mean "copy from offset 0 of
the current stack frame".

They also provide a cheap security mechanism against *miscompilations*. This
might be made optional in the future once Rune is more stable, but for now it's
helpful to detect errors early and protect the user against bad instructions.
But don't mistake it for perfect security. Like [stack protection] which is
common in modern operating systems, the mechanism can be circumvented by
malicious code. 

[stack protection]: https://en.wikipedia.org/wiki/Buffer_overflow_protection

To look close at the mechanism, let's trace the following program:

```rune
{{#include ../../scripts/book/the_stack/call_and_add.rn}}
```

```text
$> cargo run --bin rune -- scripts/book/the_stack/call_and_add.rn --trace --dump-stack
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

We're not going to go through each instruction step-by-step like in the last
section. Instead we will only examine the parts related to call frames.

We have a `call 0xbfd58656ec9a8ebe, 2` instruction, which tells the virtual
machine to jump to the function corresponding to the type hash
`0xbfd58656ec9a8ebe`, and isolate the top two values on the stack in the next
call frame.

We can see that the first argument `a` is in the *lowest* position, and the
second argument `b` is on the *highest* position. Let's examine the effects this
function call has on the stack.

```text
    0+0 = 3
    0+1 = 1
    0+2 = 2
  0008 = call 0xbfd58656ec9a8ebe, 2 // fn `foo`
=> frame 1 (1):
    1+0 = 1
    1+1 = 2
```

Here we can see a new call frame `frame 1` being allocated, and that it contains
two items: `1` and `2`.

We can also see that the items are offset from position `1`, which is the base
of the current call frame. This is shown as the addresses `1+0` and `1+1`. The
value `3` at `0+0` is no longer visible, because it is outside of the current
call frame.

Let's have a look at what happens when we `return`:

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

We call the `clean 2` instruction, which tells the vm to preserve the top of the
stack (`1+2`), and clean two items below it, leaving us with `3`. We then
`return`, which jumps us back to `frame 0`, which now has `0+0` visible *and*
our return value at `0+1`.

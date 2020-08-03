# Optimizations

## Don't store static strings on the stack

Static strings are currently copied to the stack when they are access.

Instead, introduce a `ValuePtr::StaticString(usize)` which references the slot
from the unit directly.
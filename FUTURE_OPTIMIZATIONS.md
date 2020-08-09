# Optimizations

## Don't store static strings on the stack

Static strings are currently copied to the stack when they are access.

Instead, introduce a `Value::StaticString(usize)` which references the slot
from the unit directly.

## Use less anonymous stack variables during pattern matching

Today everything that is part of a match becomes an anonymous stack variable,
this is because the "binding" happens late and we (currently) don't know up
front wheter a specific binding will be used or not.
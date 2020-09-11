# Optimizations

## Use less anonymous stack variables during pattern matching

Today everything that is part of a match becomes an anonymous stack variable,
this is because the "binding" happens late and we (currently) don't know up
front whether a specific binding will be used or not.
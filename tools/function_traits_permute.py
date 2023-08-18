# helper to generate function_traits/macros.rs

import itertools

Owned = lambda var: (var, "", "FromValue", "from_value")
Mut = lambda var: ("Mut<{}>".format(var), "&mut", "?Sized + UnsafeToMut", "unsafe_to_mut")
Ref = lambda var: ("Ref<{}>".format(var), "&", "?Sized + UnsafeToRef", "unsafe_to_ref")

vars = ["A", "B", "C", "D", "E", "F"]

print("// Note: Automatically generated using functions_traits_permute.py")
print("macro_rules! permute {")
print("    ($call:path) => {")

for repeat in range(0, 6):
    for perm in itertools.product([Owned, Ref, Mut], repeat=repeat):
        line = [str(repeat)]

        for ((n, var), p) in zip(enumerate(vars), perm):
            (ty, modifier, trait, coercion) = p(var)
            line.append("{{{}, {}, {}, {}, {{{}}}, {{{}}}, {}}}".format(var, var.lower(), ty, n, modifier, trait, coercion))

        print("        $call!({});".format(", ".join(line)))

print("    }")
print("}")

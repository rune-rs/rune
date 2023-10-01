# helper to generate function_traits/macros.rs

import itertools
import pathlib

Owned = lambda var: (var, "", ["FromValue"], "from_value")
Mut = lambda var: (f"Mut<{var}>", "&mut", ["?Sized", "UnsafeToMut"], "unsafe_to_mut")
Ref = lambda var: (f"Ref<{var}>", "&", ["?Sized", "UnsafeToRef"], "unsafe_to_ref")

vars = ['A', 'B', 'C', 'D', 'E']

this = pathlib.Path(__file__)

name = this.parts[-1]
out = this.parent.parent.joinpath("crates/rune/src/module/function_traits/macros.rs")

with out.open(mode = 'bw') as fd:
    def pr(text):
        fd.write(text.encode('utf-8'))
        fd.write(b"\n")

    pr(f"// Note: Automatically generated using {name}")
    pr("macro_rules! permute {")
    pr("    ($call:path) => {")

    for repeat in range(0, len(vars) + 1):
        for perm in itertools.product([Owned, Ref, Mut], repeat=repeat):
            line = [str(repeat)]

            for (n, (var, p)) in enumerate(zip(vars, perm)):
                (ty, mods, traits, coercion) = p(var)
                var_lower = var.lower()
                traits = " + ".join(traits)
                line.append(f"{{{var}, {var_lower}, {ty}, {n}, {{{mods}}}, {{{traits}}}, {coercion}}}")

            if repeat >= 4:
                pr("        #[cfg(not(test))]")

            args = ", ".join(line)
            pr(f"        $call!({args});")

    pr("    }")
    pr("}")

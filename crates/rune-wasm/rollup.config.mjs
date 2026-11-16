import rust from "@wasm-tool/rollup-plugin-rust";

export default {
    input: "rune.js",
    output: {
        dir: "../../site/static/js",
        format: "es",
        sourcemap: true,
    },
    plugins: [
        rust({
            extraArgs: {
                wasmOpt: ["-O", "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "--enable-sign-ext", "--enable-mutable-globals", "--enable-reference-types", "--enable-multivalue"],
            },
        }),
    ],
};

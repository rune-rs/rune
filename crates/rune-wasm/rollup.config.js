import rust from "@wasm-tool/rollup-plugin-rust";
 
export default {
    input: "rune.js",
    output: {
        dir: "../../site/static/rune",
        format: "iife",
        name: "rune",
        sourcemap: true,
    },
    plugins: [
        rust({
            serverPath: "/rune/"
        }),
    ],
};
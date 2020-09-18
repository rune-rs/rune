import rust from "@wasm-tool/rollup-plugin-rust";
 
export default {
    input: "rune.js",
    output: {
        dir: "../../site/static/play",
        format: "iife",
        name: "rune",
        sourcemap: true,
    },
    plugins: [
        rust(),
    ],
};
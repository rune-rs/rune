import init from "./rune_wasm.js";

await init({ module_or_path: new URL("./rune_wasm_bg.wasm", import.meta.url) });

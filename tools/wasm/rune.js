import init, * as module from "./rune_wasm.js";

const rune = {
    module,
    async init() {
        await init({ module_or_path: new URL("./rune_wasm_bg.wasm", import.meta.url) });
    },
};

globalThis.rune = rune;

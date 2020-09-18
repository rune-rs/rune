import wasm from "./Cargo.toml";

export async function init() {
    module = await wasm();
}

export var module = null;
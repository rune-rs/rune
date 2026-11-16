const rune = {
    module: null,
    async init() {
        this.module = await import("./Cargo.toml");
    },
};

globalThis.rune = rune;

// adapted from https://github.com/rust-analyzer/rust-analyzer/blob/ab432b36de5d6a370dbaad923f9a475a00fbf220/editors/code/src/util.ts#L16 under the MIT license.
// Copyright rust-analyzer developers.

import * as vscode from 'vscode';
import { promises as fs, PathLike } from 'fs';
import { inspect } from "util";
import { spawnSync } from "child_process";
import { strict as nativeAssert } from "assert";

/** assert wrapper that logs failures */
export function assert(condition: boolean, explanation: string): asserts condition {
    try {
        nativeAssert(condition, explanation);
    } catch (err) {
        log.error(`Assertion failed:`, explanation);
        throw err;
    }
}

/** logging plumbing */
export const log = new class {
    private enabled = true;
    private readonly output = vscode.window.createOutputChannel("Rune (Client)");

    setEnabled(yes: boolean): void {
        log.enabled = yes;
    }

    // Hint: the type [T, ...T[]] means a non-empty array
    debug(...msg: [unknown, ...unknown[]]): void {
        if (!log.enabled) return;
        log.write("DEBUG", ...msg);
    }

    info(...msg: [unknown, ...unknown[]]): void {
        log.write("INFO", ...msg);
    }

    log(...msg: [unknown, ...unknown[]]): void {
        log.write("INFO", ...msg);
    }

    warn(...msg: [unknown, ...unknown[]]): void {
        debugger;
        log.write("WARN", ...msg);
    }

    error(...msg: [unknown, ...unknown[]]): void {
        debugger;
        log.write("ERROR", ...msg);
        log.output.show(true);
    }

    private write(label: string, ...messageParts: unknown[]): void {
        const message = messageParts.map(log.stringify).join(" ");
        const dateTime = new Date().toLocaleString();
        log.output.appendLine(`${label} [${dateTime}]: ${message}`);
    }

    private stringify(val: unknown): string {
        if (typeof val === "string") return val;
        return inspect(val, {
            colors: false,
            depth: 6, // heuristic
        });
    }
};

/** Test if the given path is a valid language server executable */
export function isValidExecutable(path: string): boolean {
    log.debug("Checking availability of a binary at", path);

    const res = spawnSync(path, ["--version"], { encoding: 'utf8' });

    const printOutput = res.error && (res.error as any).code !== 'ENOENT' ? log.warn : log.debug;
    printOutput(path, "--version:", res);

    return res.status === 0;
}

/** Test if the given path exists or not */
export async function pathExists(p: PathLike): Promise<boolean> {
    return fs.stat(p).then(() => true, () => false);
}
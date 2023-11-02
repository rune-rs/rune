import * as vscode from "vscode";
import { strict as nativeAssert } from "assert";
import { exec, ExecOptions, spawnSync } from "child_process";
import { Env, substituteVariablesInEnv } from "./config";
import { inspect } from "util";

export function assert(condition: boolean, explanation: string): asserts condition {
    try {
        nativeAssert(condition, explanation);
    } catch (err) {
        log.error(`Assertion failed:`, explanation);
        throw err;
    }
}

export const log = new (class {
    private enabled = true;
    private readonly output = vscode.window.createOutputChannel("Rune Extension");

    setEnabled(yes: boolean): void {
        log.enabled = yes;
    }

    // Hint: the type [T, ...T[]] means a non-empty array
    debug(...msg: [unknown, ...unknown[]]): void {
        if (!log.enabled) {
            return;
        }
        log.write("DEBUG", ...msg);
    }

    info(...msg: [unknown, ...unknown[]]): void {
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
        if (typeof val === "string") {
            return val;
        }
        return inspect(val, {
            colors: false,
            depth: 6, // heuristic
        });
    }
})();

export function isValidExecutable(path: string, extraEnv: Env): boolean {
    log.debug("Checking availability of a binary at", path);

    const newEnv = substituteVariablesInEnv(Object.assign({}, process.env, extraEnv));
    log.debug('newEnv', newEnv);

    const res = spawnSync(path, ["--version"], { encoding: "utf8", env: newEnv });

    const printOutput = res.error && (res.error as any).code !== "ENOENT" ? log.warn : log.debug;
    printOutput(path, "--version:", res);

    return res.status === 0;
}

/** Sets ['when'](https://code.visualstudio.com/docs/getstarted/keybindings#_when-clause-contexts) clause contexts */
export function setContextValue(key: string, value: any): Thenable<void> {
    return vscode.commands.executeCommand("setContext", key, value);
}

/** Awaitable wrapper around `child_process.exec` */
export function execute(command: string, options: ExecOptions): Promise<string> {
    return new Promise((resolve, reject) => {
        exec(command, options, (err, stdout, stderr) => {
            if (err) {
                reject(err);
                return;
            }

            if (stderr) {
                reject(new Error(stderr));
                return;
            }

            resolve(stdout.trimEnd());
        });
    });
}

export async function uriExists(uri: vscode.Uri) {
    return await vscode.workspace.fs.stat(uri).then(
        () => true,
        () => false
    );
}

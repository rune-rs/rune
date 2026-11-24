import * as vscode from "vscode";
import { spawnSync } from "node:child_process";
import { inspect } from "node:util";
import { Env, substituteVariablesInEnv } from "./config";

export const log = new (class {
	private enabled = true;
	private output: vscode.LogOutputChannel | undefined;

	private getChannel(): vscode.LogOutputChannel {
		if (!this.output) {
			this.output = vscode.window.createOutputChannel("Rune Extension", { log: true });
		}
		return this.output;
	}

	setEnabled(yes: boolean): void {
		this.enabled = yes;
	}

	debug(...msg: [unknown, ...unknown[]]): void {
		if (!this.enabled) {
			return;
		}
		this.getChannel().debug(this.format(msg));
	}

	info(...msg: [unknown, ...unknown[]]): void {
		this.getChannel().info(this.format(msg));
	}

	warn(...msg: [unknown, ...unknown[]]): void {
		this.getChannel().warn(this.format(msg));
	}

	error(...msg: [unknown, ...unknown[]]): void {
		const channel = this.getChannel();
		channel.error(this.format(msg));
		channel.show(true);
	}

	dispose(): void {
		this.output?.dispose();
		this.output = undefined;
	}

	private format(messageParts: unknown[]): string {
		return messageParts.map((val) => this.stringify(val)).join(" ");
	}

	private stringify(val: unknown): string {
		if (typeof val === "string") {
			return val;
		}
		return inspect(val, {
			colors: false,
			depth: 6,
		});
	}
})();

export function isValidExecutable(path: string, extraEnv: Env): boolean {
	log.debug("Checking availability of a binary at", path);

	const newEnv = substituteVariablesInEnv({ ...process.env, ...extraEnv });
	log.debug("newEnv", newEnv);

	const res = spawnSync(path, ["--version"], { encoding: "utf8", env: newEnv });

	const errno = res.error as NodeJS.ErrnoException | undefined;
	if (errno && errno.code !== "ENOENT") {
		log.warn(path, "--version:", res);
	} else {
		log.debug(path, "--version:", res);
	}

	return res.status === 0;
}

/** Sets ['when'](https://code.visualstudio.com/docs/getstarted/keybindings#_when-clause-contexts) clause contexts */
export function setContextValue(key: string, value: unknown): Thenable<void> {
	return vscode.commands.executeCommand("setContext", key, value);
}

export async function uriExists(uri: vscode.Uri): Promise<boolean> {
	try {
		await vscode.workspace.fs.stat(uri);
		return true;
	} catch {
		return false;
	}
}

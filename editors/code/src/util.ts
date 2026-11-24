import * as vscode from "vscode";
import { execFile } from "node:child_process";
import { inspect } from "node:util";
import { type Env, substituteVariablesInEnv } from "./config";

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

	show(preserveFocus = true): void {
		this.getChannel().show(preserveFocus);
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

export async function isValidExecutable(path: string, extraEnv: Env): Promise<boolean> {
	log.debug("Checking availability of a binary at", path);

	const newEnv = substituteVariablesInEnv({ ...process.env, ...extraEnv });
	log.debug("newEnv", newEnv);

	return await new Promise<boolean>((resolve) => {
		execFile(path, ["--version"], { encoding: "utf8", env: newEnv }, (error, stdout, stderr) => {
			if (error) {
				const errno = error as NodeJS.ErrnoException;
				if (errno.code !== "ENOENT") {
					log.warn(path, "--version:", { error, stdout, stderr });
				} else {
					log.debug(path, "--version:", { error, stdout, stderr });
				}
				resolve(false);
				return;
			}
			log.debug(path, "--version:", { stdout, stderr });
			resolve(true);
		});
	});
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

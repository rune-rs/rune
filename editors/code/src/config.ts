import * as path from "node:path";
import * as vscode from "vscode";
import { log } from "./util";

export type UpdatesChannel = "nightly";

export type Env = Record<string, string | undefined>;

export class Config implements vscode.Disposable {
	readonly extensionId = "rune-vscode";
	readonly rootSection = "rune";
	readonly globalStorageUri: vscode.Uri;

	private configureLang: vscode.Disposable | undefined;

	constructor(ctx: vscode.ExtensionContext) {
		this.globalStorageUri = ctx.globalStorageUri;
		ctx.subscriptions.push(
			this,
			vscode.workspace.onDidChangeConfiguration(this.onDidChangeConfiguration, this),
		);
		this.refreshLogging();
		this.configureLanguage();
	}

	dispose(): void {
		this.configureLang?.dispose();
		this.configureLang = undefined;
	}

	private refreshLogging(): void {
		log.setEnabled(this.traceExtension);

		const cfg = Object.entries(this.cfg).filter(([, val]) => !(val instanceof Function));
		log.info("Using configuration", Object.fromEntries(cfg));
	}

	private onDidChangeConfiguration(event: vscode.ConfigurationChangeEvent): void {
		if (!event.affectsConfiguration(this.rootSection)) {
			return;
		}
		this.refreshLogging();
		this.configureLanguage();
	}

	/**
	 * Sets up additional language configuration that's impossible to do via a
	 * separate language-configuration.json file. See [1] for more information.
	 *
	 * [1]: https://github.com/Microsoft/vscode/issues/11514#issuecomment-244707076
	 */
	private configureLanguage(): void {
		if (this.configureLang) {
			this.configureLang.dispose();
			this.configureLang = undefined;
		}

		let onEnterRules: vscode.OnEnterRule[] = [
			{
				// Carry indentation from the previous line
				beforeText: /^\s*$/,
				action: { indentAction: vscode.IndentAction.None },
			},
			{
				// After the end of a function/field chain,
				// with the semicolon on the same line
				beforeText: /^\s+\..*;/,
				action: { indentAction: vscode.IndentAction.Outdent },
			},
			{
				// After the end of a function/field chain,
				// with semicolon detached from the rest
				beforeText: /^\s+;/,
				previousLineText: /^\s+\..*/,
				action: { indentAction: vscode.IndentAction.Outdent },
			},
		];

		if (this.typingContinueCommentsOnNewline) {
			const indentAction = vscode.IndentAction.None;

			onEnterRules = [
				...onEnterRules,
				{
					// Doc single-line comment
					// e.g. ///|
					beforeText: /^\s*\/{3}.*$/,
					action: { indentAction, appendText: "/// " },
				},
				{
					// Parent doc single-line comment
					// e.g. //!|
					beforeText: /^\s*\/{2}!.*$/,
					action: { indentAction, appendText: "//! " },
				},
				{
					// Begins an auto-closed multi-line comment (standard or parent doc)
					// e.g. /** | */ or /*! | */
					beforeText: /^\s*\/\*(\*|!)(?!\/)([^*]|\*(?!\/))*$/,
					afterText: /^\s*\*\/$/,
					action: {
						indentAction: vscode.IndentAction.IndentOutdent,
						appendText: " * ",
					},
				},
				{
					// Begins a multi-line comment (standard or parent doc)
					// e.g. /** ...| or /*! ...|
					beforeText: /^\s*\/\*(\*|!)(?!\/)([^*]|\*(?!\/))*$/,
					action: { indentAction, appendText: " * " },
				},
				{
					// Continues a multi-line comment
					// e.g.  * ...|
					beforeText: /^( {2})* \* (([^*]|\*(?!\/))*)?$/,
					action: { indentAction, appendText: "* " },
				},
				{
					// Dedents after closing a multi-line comment
					// e.g.  */|
					beforeText: /^( {2})* \*\/\s*$/,
					action: { indentAction, removeText: 1 },
				},
			];
		}

		this.configureLang = vscode.languages.setLanguageConfiguration("rune", {
			onEnterRules,
		});
	}

	private get cfg(): vscode.WorkspaceConfiguration {
		return vscode.workspace.getConfiguration(this.rootSection);
	}

	private get<T>(section: string): T {
		return this.cfg.get<T>(section) as T;
	}

	get serverPath(): string | null {
		return this.get<string | null>("server.path");
	}

	get serverCargoPackage(): string {
		return this.get<string>("server.cargoPackage");
	}

	get serverCargoBinary(): string {
		return this.get<string>("server.cargoBinary");
	}

	get serverExtraEnv(): Env {
		const extraEnv =
			this.get<Record<string, string | number> | null>("server.extraEnv") ?? {};
		return Object.fromEntries(
			Object.entries(extraEnv).map(([k, v]) => [k, typeof v !== "string" ? v.toString() : v]),
		);
	}

	get updatesCheckInterval(): number {
		return this.get<number>("updates.checkInterval");
	}

	get updatesAskBeforeDownload(): boolean {
		return this.get<boolean>("updates.askBeforeDownload");
	}

	get traceExtension(): boolean {
		return this.get<boolean>("trace.extension");
	}

	get typingContinueCommentsOnNewline(): boolean {
		return this.get<boolean>("typing.continueCommentsOnNewline");
	}
}

export function substituteVariablesInEnv(env: Env): Record<string, string> {
	const missingDeps = new Set<string>();
	// vscode uses `env:ENV_NAME` for env vars resolution, and it's easier
	// to follow the same convention for our dependency tracking
	const definedEntries = Object.entries(env).filter(
		(entry): entry is [string, string] => entry[1] !== undefined,
	);
	const definedEnvKeys = new Set(definedEntries.map(([key]) => `env:${key}`));
	const envWithDeps: Record<string, { deps: string[]; value: string }> = Object.fromEntries(
		definedEntries.map(([key, value]) => {
			const deps = new Set<string>();
			for (const match of value.matchAll(/\${(?<depName>.+?)}/g)) {
				const depName = match.groups?.depName;
				if (!depName) {
					continue;
				}
				deps.add(depName);
				// `depName` at this point can have a form of `expression` or
				// `prefix:expression`
				if (!definedEnvKeys.has(depName)) {
					missingDeps.add(depName);
				}
			}
			return [`env:${key}`, { deps: [...deps], value }];
		}),
	);

	const resolved = new Set<string>();
	for (const dep of missingDeps) {
		const match = /(?<prefix>.*?):(?<body>.+)/.exec(dep);
		if (match?.groups) {
			const { prefix, body } = match.groups;
			if (prefix === "env") {
				envWithDeps[dep] = {
					value: process.env[body] ?? "",
					deps: [],
				};
				resolved.add(dep);
			} else {
				// we can't handle other prefixes at the moment
				// leave values as is, but still mark them as resolved
				envWithDeps[dep] = {
					value: `\${${dep}}`,
					deps: [],
				};
				resolved.add(dep);
			}
		} else {
			envWithDeps[dep] = {
				value: computeVscodeVar(dep),
				deps: [],
			};
		}
	}
	const toResolve = new Set(Object.keys(envWithDeps));

	let leftToResolveSize: number;
	do {
		leftToResolveSize = toResolve.size;
		for (const key of toResolve) {
			if (envWithDeps[key].deps.every((dep) => resolved.has(dep))) {
				envWithDeps[key].value = envWithDeps[key].value.replace(
					/\${(?<depName>.+?)}/g,
					(_wholeMatch, depName: string) => envWithDeps[depName].value,
				);
				resolved.add(key);
				toResolve.delete(key);
			}
		}
	} while (toResolve.size > 0 && toResolve.size < leftToResolveSize);

	const resolvedEnv: Record<string, string> = {};
	for (const [key] of definedEntries) {
		resolvedEnv[key] = envWithDeps[`env:${key}`].value;
	}
	return resolvedEnv;
}

function computeVscodeVar(varName: string): string {
	// https://code.visualstudio.com/docs/editor/variables-reference
	const supportedVariables: Record<string, () => string> = {
		workspaceFolder: () => {
			const folders = vscode.workspace.workspaceFolders ?? [];
			return folders.length > 0 ? folders[0].uri.fsPath : "";
		},

		workspaceFolderBasename: () => {
			const workspaceFolder = computeVscodeVar("workspaceFolder");
			return workspaceFolder ? path.basename(workspaceFolder) : "";
		},

		cwd: () => process.cwd(),

		execPath: () => process.env.VSCODE_EXEC_PATH ?? process.execPath,

		pathSeparator: () => path.sep,
	};

	if (varName in supportedVariables) {
		return supportedVariables[varName]();
	}
	// can't resolve, keep the expression as is
	return `\${${varName}}`;
}

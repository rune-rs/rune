import * as vscode from "vscode";
import type * as lc from "vscode-languageclient/node";
import * as os from "node:os";
import * as cp from "node:child_process";
import * as rl from "node:readline";

import { log, isValidExecutable, uriExists, setContextValue } from "./util";
import { download, fetchRelease } from "./net";
import { createClient } from "./client";
import { Config } from "./config";
import { PersistentState } from "./persistent_state";

const RUNE_PROJECT_CONTEXT_NAME = "inRuneProject";

interface FoundBinary {
	path: string;
}

interface LastClientError {
	message: string;
	tooltip?: string;
}

class DetailedError extends Error {
	readonly details?: string;

	constructor(message: string, details?: string) {
		super(message);
		this.details = details;
	}
}

export class Ctx {
	readonly context: vscode.ExtensionContext;
	readonly config: Config;
	readonly state: PersistentState;
	readonly statusBar: vscode.StatusBarItem;

	// Stored initialization error. Cleared when reloaded.
	lastClientError: LastClientError | null = null;
	client: lc.LanguageClient | null = null;
	commands: Record<string, vscode.Disposable> = {};
	stopped = false;

	constructor(context: vscode.ExtensionContext) {
		this.context = context;
		this.config = new Config(context);
		this.state = new PersistentState(context.globalState);
		this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
	}

	async activate(): Promise<void> {
		// Reloading is inspired by @DanTup maneuver: https://github.com/microsoft/vscode/issues/45774#issuecomment-373423895
		this.command("rune-vscode.reload", async () => {
			this.stopped = false;
			void vscode.window.showInformationMessage("Reloading Rune Language Server...");
			await this.deactivate();
			await this.activate();
		});

		this.command("rune-vscode.stopServer", async () => {
			if (!this.stopped) {
				void vscode.window.showInformationMessage("Stopping Rune Language Server...");
				this.stopped = true;
				await this.deactivate();
			}
		});

		this.command("rune-vscode.startServer", async () => {
			if (this.stopped) {
				void vscode.window.showInformationMessage("Starting Rune Language Server...");
				this.stopped = false;
				await this.activate();
			}
		});

		if (!this.stopped) {
			await this.setupClient();
		}

		this.setupStatusBar();
	}

	command(name: string, callback: (...args: unknown[]) => unknown): void {
		if (!this.commands[name]) {
			this.commands[name] = vscode.commands.registerCommand(name, callback);
		}
	}

	async setupClient(): Promise<void> {
		this.lastClientError = null;

		try {
			const binary = await this.bootstrap();
			this.client = createClient(binary.path, this.config.serverExtraEnv, ["language-server"]);
			await this.client.start();
			await setContextValue(RUNE_PROJECT_CONTEXT_NAME, true);
		} catch (err: unknown) {
			const message =
				"Bootstrap error! " +
				'See the logs in "OUTPUT > Rune Extension" (should open automatically). ' +
				'To enable verbose logs use { "rune.trace.extension": true }';
			void vscode.window.showErrorMessage(message);

			this.lastClientError = {
				message: err instanceof Error ? err.message : String(err),
				tooltip: err instanceof DetailedError ? err.details : undefined,
			};

			log.error(err);
		}
	}

	async bootstrap(): Promise<FoundBinary> {
		const binary = await this.getServer();

		if (!binary) {
			throw new Error("Rune Language Server is not available.");
		}

		log.info("Using server binary at", binary.path);

		if (!isValidExecutable(binary.path, this.config.serverExtraEnv)) {
			if (this.config.serverPath) {
				throw new Error(
					`Failed to execute ${binary.path} --version. \`rune.server.path\` has been set explicitly. ` +
						"Consider removing this config or making a valid server binary available at that path.",
				);
			} else if (this.config.serverCargoPackage) {
				throw new Error(
					`Failed to execute ${binary.path} --version. \`rune.server.cargoPackage\` has been set explicitly. ` +
						"Consider removing this config or making a valid cargo package.",
				);
			} else {
				throw new Error(`Failed to execute ${binary.path} --version`);
			}
		}

		return binary;
	}

	/**
	 * Run cargo --version to determine if the command is available to begin with.
	 *
	 * @returns stdout from cargo --version
	 */
	async cargoVersion(): Promise<string> {
		const cargoVersion = cp.spawn("cargo", ["--version"]);
		cargoVersion.stdout.setEncoding("utf-8");

		const [code, out, err] = await new Promise<[number | null, string, string]>(
			(resolve, reject) => {
				let stdout = "";
				let stderr = "";

				cargoVersion.stdout.on("data", (chunk: string) => {
					stdout += chunk;
				});

				cargoVersion.stderr.on("data", (chunk: Buffer | string) => {
					stderr += chunk.toString();
				});

				cargoVersion.on("error", reject);

				cargoVersion.on("exit", (exitCode) => {
					resolve([exitCode, stdout, stderr]);
				});
			},
		);

		if (code !== 0) {
			throw new DetailedError(`cargo --version: failed (${code})`, err);
		}

		return out.trim();
	}

	/**
	 * @param name package to build.
	 * @param binary optional binary target name.
	 * @returns a string to the path of the built binary.
	 */
	async buildCargoPackage(name: string, binary: string): Promise<string | null> {
		interface Target {
			kind: string[];
			name: string;
		}

		interface ReasonOutput {
			reason: string;
		}

		interface CompilerArtifact extends ReasonOutput {
			package_id: string;
			target: Target;
			executable?: string;
		}

		const cargo = await this.cargoVersion();
		log.info(`Cargo: ${cargo}`);

		if (!vscode.workspace.workspaceFolders || vscode.workspace.workspaceFolders.length === 0) {
			throw new Error("No workspace folders");
		}

		const folder = vscode.workspace.workspaceFolders[0];
		log.info(folder.uri.fsPath);

		const child = cp.spawn("cargo", ["build", "-p", name, "--message-format", "json"], {
			cwd: folder.uri.fsPath,
		});
		child.stderr.setEncoding("utf8");
		child.stdout.setEncoding("utf8");
		const out = rl.createInterface(child.stdout, child.stdin);

		this.statusBar.text = `rune: cargo build -p ${name}`;
		this.statusBar.tooltip = `rune: building package ${name}, to use as rune language server`;
		this.statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
		this.statusBar.show();

		let executable: string | null = null;
		let error = "";

		child.stderr.on("data", (data: string) => {
			error += data;
		});

		const code = await new Promise<number | null>((resolve, reject) => {
			out.on("line", (data) => {
				log.debug(data);

				let output: ReasonOutput;
				try {
					output = JSON.parse(data) as ReasonOutput;
				} catch {
					return;
				}

				if (output.reason === "compiler-artifact") {
					const artifact = output as CompilerArtifact;

					this.statusBar.text = `rune: cargo (${artifact.target.name})`;

					if (artifact.target.kind.includes("bin")) {
						if (binary === "" || artifact.target.name === binary) {
							executable = artifact.executable ?? null;
						}
					}
				}
			});

			out.on("close", () => {
				log.debug("Closed");
			});

			child.on("error", reject);

			child.on("exit", (exitCode) => {
				resolve(exitCode);
			});
		});

		if (code !== 0) {
			throw new DetailedError(`cargo build -p ${name}: failed (${code})`, error);
		}

		this.statusBar.hide();

		if (!executable) {
			log.info("No executable");
			return null;
		}

		log.info(`Executable: ${executable}`);
		return executable;
	}

	async getServer(): Promise<FoundBinary | null> {
		// use explicit path from config
		const explicitPath = this.serverPath();

		if (explicitPath) {
			if (explicitPath.startsWith("~/")) {
				return { path: os.homedir() + explicitPath.slice("~".length) };
			}

			return { path: explicitPath };
		}

		const cargoPackage = this.config.serverCargoPackage;

		if (cargoPackage) {
			const builtPath = await this.buildCargoPackage(cargoPackage, this.config.serverCargoBinary);
			return builtPath ? { path: builtPath } : null;
		}

		const downloadedPath = await this.downloadServer();
		return downloadedPath ? { path: downloadedPath } : null;
	}

	async downloadServer(): Promise<string | null> {
		const platformArch = detectPlatform();

		if (!platformArch) {
			return null;
		}

		const [platform, arch] = platformArch;

		// platform specific binary name / path
		const ext = platform === "windows" ? ".exe" : "";
		const bin = `rune-languageserver-${platform}${ext}`;
		const serverDir = vscode.Uri.joinPath(this.context.extensionUri, "server");
		const dest = vscode.Uri.joinPath(serverDir, bin);
		const destExists = await uriExists(dest);

		// Only check for updates once every two hours.
		const now = Date.now() / 1000;
		const lastCheck = this.state.lastCheck;
		const timedOut = !lastCheck || now - lastCheck > this.config.updatesCheckInterval;
		log.debug("Check cache timeout", {
			now,
			lastCheck,
			timedOut,
			timeout: this.config.updatesCheckInterval,
		});

		if (destExists && !timedOut) {
			return dest.fsPath;
		}

		// fetch new release info
		await this.state.updateLastCheck(now);

		const release = await fetchRelease("nightly", null);

		const platformOnly = `rune-languageserver-${platform}.gz`;
		const platformArchName = `rune-languageserver-${platform}-${arch}.gz`;

		const artifact = release.assets.find(
			(a) => a.name === platformOnly || a.name === platformArchName,
		);

		if (!artifact) {
			log.warn(`Bad release: ${JSON.stringify(release)}`);
			return null;
		}

		// no new release
		if (destExists && this.state.releaseId === artifact.id) {
			return dest.fsPath;
		}

		// ask for update
		if (this.config.updatesAskBeforeDownload) {
			const userResponse = await vscode.window.showInformationMessage(
				`A new version of the Rune Language Server is available (asset id: ${artifact.id}).`,
				"Download now",
			);
			if (userResponse !== "Download now") {
				return destExists ? dest.fsPath : null;
			}
		}

		// delete old server version
		try {
			await vscode.workspace.fs.delete(dest);
		} catch (exception) {
			log.debug("Delete of old server binary failed", exception);
		}

		// create server dir if missing
		if (!(await uriExists(serverDir))) {
			log.debug(`Creating server dir: ${serverDir}`);
			await vscode.workspace.fs.createDirectory(serverDir);
		}

		// download new version
		await download({
			url: artifact.browser_download_url,
			dest,
			progressTitle: "Downloading Rune Language Server",
			gunzip: true,
			mode: 0o755,
		});

		await this.state.updateReleaseId(artifact.id);
		return dest.fsPath;
	}

	serverPath(): string | null {
		return process.env.__RUNE_LSP_SERVER_DEBUG ?? this.config.serverPath;
	}

	setupStatusBar(): void {
		this.statusBar.text = "rune";

		let tooltipExtra: string;

		if (this.lastClientError) {
			this.statusBar.text += `: ${this.lastClientError.message}`;

			if (this.lastClientError.tooltip) {
				tooltipExtra =
					"#### Error\n\n" +
					this.lastClientError.tooltip
						.split("\n")
						.map((s) => s.trim())
						.join("\n");
			} else {
				tooltipExtra = "#### Error";
			}

			this.statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
		} else {
			tooltipExtra = "rune language server";
			this.statusBar.backgroundColor = undefined;
		}

		this.statusBar.command = "rune-vscode.reload";

		const tooltip = new vscode.MarkdownString("", true);
		tooltip.isTrusted = true;

		tooltip.appendMarkdown("\n\n[Restart server](command:rune-vscode.reload)");

		if (this.lastClientError) {
			if (!this.stopped) {
				tooltip.appendMarkdown("\n\n[Stop server](command:rune-vscode.stopServer)");
			} else {
				tooltip.appendMarkdown("\n\n[Start server](command:rune-vscode.startServer)");
			}
		}

		if (tooltipExtra) {
			tooltip.appendMarkdown(`\n\n${tooltipExtra}`);
		}

		this.statusBar.tooltip = tooltip;
		this.statusBar.show();
	}

	/**
	 * Partial disposal, where we only dispose of the client and any handlers
	 * which are related to the current project. This happens as part of a
	 * reload.
	 */
	async deactivate(): Promise<void> {
		if (this.client !== null) {
			await setContextValue(RUNE_PROJECT_CONTEXT_NAME, undefined);
			await this.client.stop();
			this.client = null;
		}

		this.setupStatusBar();
	}

	/**
	 * Full disposal, where we completely work towards deactivating the
	 * extension.
	 */
	async dispose(): Promise<void> {
		await this.deactivate();

		for (const disposable of Object.values(this.commands)) {
			disposable.dispose();
		}

		this.commands = {};
		this.statusBar.dispose();
	}
}

/**
 * Function used to detect the platform we are on.
 */
function detectPlatform(): [string, string] | undefined {
	let platform: string | undefined;
	let arch: string | undefined;

	switch (process.platform) {
		case "win32":
			platform = "windows";
			break;
		case "linux":
			platform = "linux";
			break;
		case "darwin":
			platform = "macos";
			break;
	}

	switch (process.arch) {
		case "x64":
			arch = "x86_64";
			break;
		case "arm64":
			arch = "aarch64";
			break;
	}

	if (!platform || !arch) {
		void vscode.window.showErrorMessage(
			`Unfortunately we don't support your platform yet. ` +
				`You can open an issue about that [here](https://github.com/rune-rs/rune/issues). ` +
				`Please include (platform: ${process.platform}, arch: ${process.arch}).`,
		);

		return undefined;
	}

	return [platform, arch];
}

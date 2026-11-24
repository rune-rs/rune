import * as vscode from "vscode";
import { type LanguageClient, State } from "vscode-languageclient/node";
import * as os from "node:os";
import * as cp from "node:child_process";
import * as fs from "node:fs";
import * as rl from "node:readline";

import { log, isValidExecutable, uriExists, setContextValue } from "./util";
import { download, fetchRelease, type GithubRelease } from "./net";
import { createClient } from "./client";
import { outputChannel } from "./extension";
import { Config } from "./config";
import { PersistentState } from "./persistent_state";

const RUNE_PROJECT_CONTEXT_NAME = "inRuneProject";

type ServerState = "starting" | "running" | "stopped" | "error";

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

	// Stored initialization or crash error. Cleared when reloaded.
	lastClientError: LastClientError | null = null;
	client: LanguageClient | null = null;
	clientStateSubscription: vscode.Disposable | null = null;
	serverConfigSubscription: vscode.Disposable | null = null;
	commands: Record<string, vscode.Disposable> = {};
	stopped = false;
	serverState: ServerState = "stopped";

	constructor(context: vscode.ExtensionContext) {
		this.context = context;
		this.config = new Config(context);
		this.state = new PersistentState(context.globalState);
		this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
	}

	async activate(): Promise<void> {
		// Reloading is inspired by @DanTup maneuver: https://github.com/microsoft/vscode/issues/45774#issuecomment-373423895
		this.command("rune-vscode.reload", async () => {
			await this.reload();
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

		this.command("rune-vscode.openLogs", () => {
			log.show(false);
		});

		this.command("rune-vscode.statusBarMenu", async () => {
			await this.showStatusBarMenu();
		});

		if (!this.serverConfigSubscription) {
			this.serverConfigSubscription = this.config.onDidChangeServerConfig(() => {
				void this.promptServerConfigReload();
			});
		}

		if (!this.stopped) {
			await this.setupClient();
		} else {
			this.serverState = "stopped";
		}

		this.setupStatusBar();
	}

	private async reload(): Promise<void> {
		this.stopped = false;
		void vscode.window.showInformationMessage("Reloading Rune Language Server...");
		await this.deactivate();
		await this.activate();
	}

	private async promptServerConfigReload(): Promise<void> {
		const choice = await vscode.window.showInformationMessage(
			"Rune server configuration changed. Reload the language server to apply changes?",
			"Reload",
		);
		if (choice === "Reload") {
			await this.reload();
		}
	}

	private async showStatusBarMenu(): Promise<void> {
		interface MenuItem extends vscode.QuickPickItem {
			action: () => Promise<void> | void;
		}

		const items: MenuItem[] = [
			{
				label: "$(refresh) Restart Language Server",
				action: () => this.reload(),
			},
			this.stopped
				? {
						label: "$(play) Start Language Server",
						action: async () => {
							await vscode.commands.executeCommand("rune-vscode.startServer");
						},
					}
				: {
						label: "$(debug-stop) Stop Language Server",
						action: async () => {
							await vscode.commands.executeCommand("rune-vscode.stopServer");
						},
					},
			{
				label: "$(output) Show Language Server Output",
				action: () => {
					outputChannel().show(false);
				},
			},
			{
				label: "$(list-unordered) Show Extension Logs",
				action: () => {
					log.show(false);
				},
			},
			{
				label: "$(settings-gear) Open Rune Settings",
				action: async () => {
					await vscode.commands.executeCommand("workbench.action.openSettings", "rune");
				},
			},
		];

		const selected = await vscode.window.showQuickPick(items, {
			placeHolder: "Rune Language Server",
		});
		if (selected) {
			await selected.action();
		}
	}

	command(name: string, callback: (...args: unknown[]) => unknown): void {
		if (!this.commands[name]) {
			this.commands[name] = vscode.commands.registerCommand(name, callback);
		}
	}

	async setupClient(): Promise<void> {
		this.lastClientError = null;
		this.serverState = "starting";
		this.setupStatusBar();

		try {
			const binary = await this.bootstrap();
			const client = createClient(binary.path, this.config.serverExtraEnv, ["language-server"]);
			this.client = client;

			this.clientStateSubscription?.dispose();
			this.clientStateSubscription = client.onDidChangeState((event) => {
				switch (event.newState) {
					case State.Starting:
						this.serverState = "starting";
						break;
					case State.Running:
						this.lastClientError = null;
						this.serverState = "running";
						break;
					case State.Stopped:
						if (this.stopped) {
							this.serverState = "stopped";
						} else {
							this.serverState = "error";
							this.lastClientError ??= {
								message: "Language server stopped unexpectedly.",
							};
						}
						break;
				}
				this.setupStatusBar();
			});

			await client.start();
			this.serverState = "running";
			await setContextValue(RUNE_PROJECT_CONTEXT_NAME, true);
		} catch (err: unknown) {
			this.serverState = "error";
			this.lastClientError = {
				message: err instanceof Error ? err.message : String(err),
				tooltip: err instanceof DetailedError ? err.details : undefined,
			};
			this.setupStatusBar();
			log.error(err);

			const message = `Failed to start Rune Language Server: ${this.lastClientError.message}`;
			void vscode.window
				.showErrorMessage(message, "Show Output", "Retry", "Open Settings")
				.then(async (choice) => {
					if (choice === "Show Output") {
						log.show(false);
					} else if (choice === "Retry") {
						await this.reload();
					} else if (choice === "Open Settings") {
						await vscode.commands.executeCommand("workbench.action.openSettings", "rune.server");
					}
				});
		}
	}

	async bootstrap(): Promise<FoundBinary> {
		const binary = await this.getServer();

		if (!binary) {
			throw new Error("Rune Language Server is not available.");
		}

		log.info("Using server binary at", binary.path);

		if (!(await isValidExecutable(binary.path, this.config.serverExtraEnv))) {
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

		return await vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				title: `Building Rune Language Server (cargo build -p ${name})`,
				cancellable: true,
			},
			async (progress, token) => {
				const child = cp.spawn("cargo", ["build", "-p", name, "--message-format", "json"], {
					cwd: folder.uri.fsPath,
				});
				child.stderr.setEncoding("utf8");
				child.stdout.setEncoding("utf8");
				const out = rl.createInterface(child.stdout, child.stdin);

				const cancelListener = token.onCancellationRequested(() => {
					child.kill();
				});

				let executable: string | null = null;
				let error = "";

				child.stderr.on("data", (data: string) => {
					error += data;
				});

				let code: number | null;
				try {
					code = await new Promise<number | null>((resolve, reject) => {
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

								progress.report({ message: artifact.target.name });

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
				} finally {
					cancelListener.dispose();
					out.close();
				}

				if (token.isCancellationRequested) {
					throw new Error("Cargo build cancelled by user.");
				}

				if (code !== 0) {
					throw new DetailedError(`cargo build -p ${name}: failed (${code})`, error);
				}

				if (!executable) {
					log.info("No executable");
					return null;
				}

				log.info(`Executable: ${executable}`);
				return executable;
			},
		);
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

		// Store downloaded binaries in globalStorageUri so they survive extension updates.
		const ext = platform === "windows" ? ".exe" : "";
		const bin = `rune-${platform}${ext}`;
		const serverDir = vscode.Uri.joinPath(this.config.globalStorageUri, "server");
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
			if (process.platform !== "win32") {
				await fs.promises.chmod(dest.fsPath, 0o755);
			}
			return dest.fsPath;
		}

		// Fetch release metadata; if offline/rate-limited, fall back to an existing binary.
		let release: GithubRelease;
		try {
			release = await fetchRelease("nightly", null);
		} catch (err) {
			if (destExists) {
				log.warn("Failed to check for updates; using cached server binary:", err);
				if (process.platform !== "win32") {
					await fs.promises.chmod(dest.fsPath, 0o755);
				}
				return dest.fsPath;
			}
			throw err;
		}

		await this.state.updateLastCheck(now);

		const gzPlatformArch = `rune-${platform}-${arch}.gz`;
		const gzPlatformOnly = `rune-${platform}.gz`;
		const zipPlatformArch = `rune-nightly-${platform}-${arch}.zip`;

		const artifact =
			release.assets.find((a) => a.name === gzPlatformArch) ??
			release.assets.find((a) => a.name === gzPlatformOnly) ??
			release.assets.find((a) => a.name === zipPlatformArch);

		if (!artifact) {
			log.warn(`Bad release: ${JSON.stringify(release)}`);
			return destExists ? dest.fsPath : null;
		}

		// no new release
		if (destExists && this.state.releaseId === artifact.id) {
			if (process.platform !== "win32") {
				await fs.promises.chmod(dest.fsPath, 0o755);
			}
			return dest.fsPath;
		}

		// ask for update
		if (this.config.updatesAskBeforeDownload) {
			const userResponse = await vscode.window.showInformationMessage(
				`A new version of Rune is available (asset id: ${artifact.id}).`,
				"Download now",
			);
			if (userResponse !== "Download now") {
				return destExists ? dest.fsPath : null;
			}
		}

		// create server dir if missing
		if (!(await uriExists(serverDir))) {
			log.debug(`Creating server dir: ${serverDir}`);
			await vscode.workspace.fs.createDirectory(serverDir);
		}

		const isZip = artifact.name.endsWith(".zip");

		// download new version
		await download({
			url: artifact.browser_download_url,
			dest,
			progressTitle: "Downloading Rune",
			gunzip: !isZip,
			zipEntry: isZip ? `rune${ext}` : undefined,
			mode: 0o755,
		});

		await this.state.updateReleaseId(artifact.id);
		return dest.fsPath;
	}

	serverPath(): string | null {
		return process.env.__RUNE_LSP_SERVER_DEBUG ?? this.config.serverPath;
	}

	setupStatusBar(): void {
		let tooltipExtra: string;

		switch (this.serverState) {
			case "starting":
				this.statusBar.text = "$(sync~spin) rune";
				this.statusBar.backgroundColor = undefined;
				tooltipExtra = "Rune language server is starting...";
				break;
			case "running":
				this.statusBar.text = "$(check) rune";
				this.statusBar.backgroundColor = undefined;
				tooltipExtra = "Rune language server is running";
				break;
			case "stopped":
				this.statusBar.text = "$(debug-stop) rune";
				this.statusBar.backgroundColor = undefined;
				tooltipExtra = "Rune language server is stopped";
				break;
			case "error": {
				this.statusBar.text = "$(error) rune";
				this.statusBar.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
				const errMsg = this.lastClientError?.message ?? "Unknown error";
				if (this.lastClientError?.tooltip) {
					const details = this.lastClientError.tooltip
						.split("\n")
						.map((s) => s.trim())
						.join("\n");
					tooltipExtra = `#### Error\n\n${errMsg}\n\n${details}`;
				} else {
					tooltipExtra = `#### Error\n\n${errMsg}`;
				}
				break;
			}
		}

		this.statusBar.command = "rune-vscode.statusBarMenu";

		const tooltip = new vscode.MarkdownString("", true);
		tooltip.isTrusted = true;

		tooltip.appendMarkdown(tooltipExtra);
		tooltip.appendMarkdown("\n\n---\n\n");
		tooltip.appendMarkdown("[Restart server](command:rune-vscode.reload)");

		if (this.stopped) {
			tooltip.appendMarkdown(" | [Start server](command:rune-vscode.startServer)");
		} else {
			tooltip.appendMarkdown(" | [Stop server](command:rune-vscode.stopServer)");
		}

		tooltip.appendMarkdown(" | [Open logs](command:rune-vscode.openLogs)");

		this.statusBar.tooltip = tooltip;
		this.statusBar.show();
	}

	/**
	 * Partial disposal, where we only dispose of the client and any handlers
	 * which are related to the current project. This happens as part of a
	 * reload.
	 */
	async deactivate(): Promise<void> {
		this.clientStateSubscription?.dispose();
		this.clientStateSubscription = null;

		if (this.client !== null) {
			await setContextValue(RUNE_PROJECT_CONTEXT_NAME, undefined);
			await this.client.stop();
			this.client = null;
		}

		this.serverState = "stopped";
		this.setupStatusBar();
	}

	/**
	 * Full disposal, where we completely work towards deactivating the
	 * extension.
	 */
	async dispose(): Promise<void> {
		await this.deactivate();

		this.serverConfigSubscription?.dispose();
		this.serverConfigSubscription = null;

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

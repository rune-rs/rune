import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import * as os from "os";
import * as cp from "child_process";
import * as rl from "readline";

import { log, isValidExecutable, assert, uriExists, setContextValue } from "./util";
import { download, fetchRelease } from "./net";
import { createClient } from "./client";
import { Config } from "./config";
import { PersistentState } from "./persistent_state";

const RUNE_PROJECT_CONTEXT_NAME = "inRuneProject";

interface FoundBinary {
    kind: "languageserver" | "cli",
    path: string
}

interface LastClientError {
    message: string,
    tooltip?: string,
}

export class Ctx {
    readonly context: vscode.ExtensionContext;
    readonly config: Config;
    readonly state: PersistentState; 
    readonly statusBar: vscode.StatusBarItem;
    
    // Stored initialization error. Cleared when reloaded.
    lastClientError: LastClientError | null;
    client: lc.LanguageClient | null;
    commands: {[key:string]: lc.Disposable};
    stopped: boolean;

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
        this.config = new Config(context);
        this.state = new PersistentState(context.globalState);    
        this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
        this.lastClientError = null;
        this.client = null;
        this.commands = {};
        this.stopped = false;
    }

    async activate() {
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

    command(name: string, callback: (...args: any[]) => any) {
        if (!this.commands[name]) {
            this.commands[name] = vscode.commands.registerCommand(name, callback);
        }
    }

    async setupClient() {
        this.lastClientError = null;

        try {
            const binary = await this.bootstrap();

            switch (binary.kind) {
                case "languageserver":
                    this.client = createClient(binary.path, this.config.serverExtraEnv);
                    break;
                case "cli":
                    this.client = createClient(binary.path, this.config.serverExtraEnv, ["language-server"]);
                    break;
            }

            await this.client.start();
            await setContextValue(RUNE_PROJECT_CONTEXT_NAME, true);
        } catch (err: any) {
            let message = "Bootstrap error! ";
            message += 'See the logs in "OUTPUT > Rune" (should open automatically). ';
            message += 'To enable verbose logs use { "rune-vscode.trace.extension": true }';
            vscode.window.showErrorMessage(message);

            this.lastClientError = {
                message: err.toString(),
                tooltip: err.details && err.details as string || null,
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
                throw new Error(`Failed to execute ${binary.path} --version. \`config.server.path\` has been set explicitly.\
                Consider removing this config or making a valid server binary available at that path.`);
            } else if (this.config.serverCargoPackage) {
                throw new Error(`Failed to execute ${binary.path} --version. \`config.server.package\` has been set explicitly.\
                Consider removing this config or making a valid cargo package.`);
            } else {
                throw new Error(`Failed to execute ${binary.path} --version`);
            }
        }

        return binary;
    }

    /**
     * Run cargo --version to determine if the command is available to begin
     * with.
     *
     * @returns stdout from cargo --version
     */
    async cargoVersion(): Promise<string> {
        let cargoVersion = cp.spawn("cargo", ["--version"]);
        cargoVersion.stdout.setEncoding('utf-8');

        let [code, out, err]: [number | null, string, string] = await new Promise((resolve, _) => {
            let stdout = "";
            let stderr = "";
    
            cargoVersion.stdout.on("data", (chunk) => {
                stdout += chunk;
            });
    
            cargoVersion.stderr.on("data", (chunk) => {
                stderr += chunk;
            });

            cargoVersion.on("exit", (code) => {
                resolve([code, stdout, stderr]);
            });
        });

        if (code !== 0) {
            let e = new Error(`cargo --version: failed (${code})`);
            // Smuggle details.
            (e as any).details = err;
            throw e;
        }

        return out.trim();
    }

    /**
     * 
     * @param name package to build.
     * @returns a string to the path of the built binary.
     */
    async buildCargoPackage(name: string): Promise<string | null> {
        interface Target {
            kind: [string],
            name: string,
        }

        interface ReasonOutput {
            reason: string,
        }

        interface CompilerArtifact extends ReasonOutput {
            package_id: string,
            target: Target,
            executable?: string,
        }

        let cargo = await this.cargoVersion();
        log.info(`Cargo: ${cargo}`);

        if (!vscode.workspace.workspaceFolders) {
            throw new Error("No workspace folders");
        }

        let folder = vscode.workspace.workspaceFolders[0];
        log.info(folder.uri.fsPath);

        let child = cp.spawn("cargo", ["build", "-p", name, "--message-format", "json"], { cwd: folder.uri.fsPath });
        child.stderr.setEncoding('utf8');
        child.stdout.setEncoding('utf8');
        let out = rl.createInterface(child.stdout, child.stdin);
    
        this.statusBar.text = `rune: cargo build -p ${name}`;
        this.statusBar.tooltip = `rune: building package ${name}, to use as rune language server`;
        this.statusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        this.statusBar.show();

        let executable = null;
        let error = "";

        child.stderr.on('data', (data) => {
            error += data;
        });

        let code = await new Promise((resolve, reject) => {
            out.on('line', (data) => {
                log.debug(data);

                let output = JSON.parse(data) as ReasonOutput;

                if (output.reason === "compiler-artifact") {
                    let artifact = output as CompilerArtifact;
                    this.statusBar.text = `rune: cargo (${artifact.target.name})`;

                    let [id, ...rest] = artifact.package_id.split(" ");

                    if (id === name && artifact.target.kind.includes("bin")) {
                        executable = artifact.executable;
                    }
                }
            });

            out.on('close', () => {
                log.debug("Closed");
            });

            child.on("exit", (code) => {
                resolve(code);
            });
        });

        if (code !== 0) {
            let e = new Error(`cargo build -p ${name}: failed (${code})`);
            // Smuggle details.
            (e as any).details = error;
            throw e;
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
                return { kind: "languageserver", path: os.homedir() + explicitPath.slice("~".length) };
            }

            return { kind: "languageserver", path: explicitPath };
        }

        const cargoPackage = this.config.serverCargoPackage;

        if (cargoPackage) {
            let path = await this.buildCargoPackage(cargoPackage);

            if (!path) {
                return null;
            }

            return { kind: "cli", path };
        }

        let path = await this.downloadServer();

        if (!path) {
            return null;
        }

        return { kind: "languageserver", path };
    }
    
    async downloadServer(): Promise<string | null> {
        // unknown platform => no download available
        const platform = detectPlatform();

        if (!platform) {
            return null;
        }
    
        // platform specific binary name / path
        const ext = platform === "windows" ? ".exe" : "";
        const bin = `rune-languageserver-${platform}${ext}`;
        const serverDir = vscode.Uri.joinPath(this.context.extensionUri, "server");
        const dest = vscode.Uri.joinPath(serverDir, bin);
        const destExists = await uriExists(dest);
    
        // Only check for updates once every two hours.
        let now = (new Date()).getTime() / 1000;
        let lastCheck = this.state.lastCheck;
        let timedOut = !lastCheck || (now - lastCheck) > this.config.updatesCheckInterval;
        log.debug("Check cache timeout", { now, lastCheck, timedOut, timeout: this.config.updatesCheckInterval });

        if (destExists && !timedOut) {
            return dest.fsPath;
        }
    
        // fetch new release info
        await this.state.updateLastCheck(now);

        const release = await fetchRelease("nightly", null);
        const artifact = release.assets.find(artifact => artifact.name === `rune-languageserver-${platform}.gz`);
        assert(!!artifact, `Bad release: ${JSON.stringify(release)}`);
    
        // no new release
        if (destExists && this.state.releaseId === artifact.id) {
            return dest.fsPath;
        }
    
        // ask for update
        if (this.config.updatesAskBeforeDownload) {
            const userResponse = await vscode.window.showInformationMessage(
                `A new version of the Rune Language Server is available (asset id: ${artifact.id}).`,
                "Download now"
            );
            if (userResponse !== "Download now") {
                return dest.fsPath;
            }
        }
    
        // delete old server version
        try {
            await vscode.workspace.fs.delete(dest);
        } catch (exception) {
            log.debug("Delete of old server binary failed", exception);
        }
    
        // create server dir if missing
        if (!await uriExists(serverDir)) {
            log.debug(`Creating server dir: ${serverDir}`);
            await vscode.workspace.fs.createDirectory(serverDir);
        }
    
        // download new version
        await download({
            url: artifact.browser_download_url,
            dest,
            progressTitle: "Downloading Rune Language Server",
            gunzip: true,
            mode: 0o755
        });

        await this.state.updateReleaseId(release.id);
        return dest.fsPath;
    }   
 
    serverPath(): string | null {
        return process.env.__RUNE_LSP_SERVER_DEBUG ?? this.config.serverPath;
    } 

    setupStatusBar() {
        this.statusBar.text = "rune";

        let tooltipExtra = null;

        if (!!this.lastClientError) {
            this.statusBar.text += ": " + this.lastClientError.message;

            if (this.lastClientError.tooltip) {
                tooltipExtra = "#### Error\n\n" + this.lastClientError.tooltip.split("\n").map((s) => s.trim()).join("\n");
            } else {
                tooltipExtra = "#### Error";
            }

            this.statusBar.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        } else {
            tooltipExtra = "rune language server";
            this.statusBar.backgroundColor = undefined;
        }

        this.statusBar.command = "rune-vscode.reload";

        let tooltip = new vscode.MarkdownString("", true);
        tooltip.isTrusted = true;

        tooltip.appendMarkdown("\n\n[Restart server](command:rune-vscode.reload)");

        if (!!this.lastClientError) {
            if (!this.stopped) {
                tooltip.appendMarkdown("\n\n[Stop server](command:rune-vscode.stopServer)");
            } else {
                tooltip.appendMarkdown("\n\n[Start server](command:rune-vscode.startServer)");
            }
        }

        if (tooltipExtra) {
            tooltip.appendMarkdown("\n\n" + tooltipExtra);
        }

        this.statusBar.tooltip = tooltip;
        this.statusBar.show();
    }

    /**
     * Partial disposal, where we only dispose of the client and any handlers
     * which are related to the current project. This happens as part of a
     * reload.
     */
    async deactivate() {
        if (this.client !== null) {
            await setContextValue(RUNE_PROJECT_CONTEXT_NAME, undefined);
            await this.client?.stop();
            this.client = null;
        }

        this.setupStatusBar();
    }

    /**
     * Full disposal, where we completely work towards deactivating the
     * extension.
     */
    async dispose() {
        await this.deactivate();

        for (let name in this.commands) {
            this.commands[name].dispose();
        }

        this.commands = {};
        this.statusBar.hide();
    }
}

/**
 * Function used to detect the platform we are on.
 */
function detectPlatform(): String | undefined {
	if (process.arch === "x64") {
		switch (process.platform) {
			case "win32":
				return "windows";
			case "linux":
				return "linux";
			case "darwin":
				return "macos";
		}
	}

	vscode.window.showErrorMessage(
		`Unfortunately we don't support your platform yet.
            You can open an issue about that [here](https://github.com/rune-rs/rune/issues).
            Please include (platform: ${process.platform}, arch: ${process.arch}).`
	);
	return undefined;
}

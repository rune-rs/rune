import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import * as os from "os";

import { log, isValidExecutable, assert, uriExists, setContextValue } from "./util";
import { download, fetchRelease } from "./net";
import { createClient } from "./client";
import { Config } from "./config";
import { PersistentState } from "./persistent_state";

const RUNE_PROJECT_CONTEXT_NAME = "inRuneProject";

export class Ctx {
    readonly context: vscode.ExtensionContext;
    readonly config: Config;
    readonly state: PersistentState; 
    readonly statusBar: vscode.StatusBarItem;

    client: lc.LanguageClient | null;
    commands: {[key:string]: lc.Disposable};
    stopped: boolean;

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
        this.config = new Config(context);
        this.state = new PersistentState(context.globalState);    
        this.statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left);
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
        try {
            const serverPath = await this.bootstrap();
            this.client = createClient(serverPath, this.config.serverExtraEnv);
            await this.client.start();
            await setContextValue(RUNE_PROJECT_CONTEXT_NAME, true);
        } catch(err) {
            let message = "Bootstrap error! ";
            message += 'See the logs in "OUTPUT > Rune" (should open automatically). ';
            message += 'To enable verbose logs use { "rune-vscode.trace.extension": true }';
            vscode.window.showErrorMessage(message);
            log.error(err);
        }
    }

    async bootstrap(): Promise<string> {
        const path = await this.getServer();

        if (!path) {
            throw new Error("Rune Language Server is not available.");
        }

        log.info("Using server binary at", path);

        if (!isValidExecutable(path)) {
            if (this.config.serverPath) {
                throw new Error(`Failed to execute ${path} --version. \`config.server.path\` has been set explicitly.\
                Consider removing this config or making a valid server binary available at that path.`);
            } else {
                throw new Error(`Failed to execute ${path} --version`);
            }
        }

        return path;
    }

    async getServer(): Promise<string | undefined> {
        // use explicit path from config
        const explicitPath = this.serverPath();

        if (explicitPath) {
            if (explicitPath.startsWith("~/")) {
                return os.homedir() + explicitPath.slice("~".length);
            }

            return explicitPath;
        }
    
        // unknown platform => no download available
        const platform = detectPlatform();
        if (!platform) {
            return undefined;
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
        this.statusBar.show();
        this.statusBar.text = "rune";
        this.statusBar.command = "rune-vscode.reload";

        let tooltip = new vscode.MarkdownString("", true);
        tooltip.isTrusted = true;

        tooltip.appendMarkdown("\n\n[Restart server](command:rune-vscode.reload)");

        if (!this.stopped) {
            tooltip.appendMarkdown("\n\n[Stop server](command:rune-vscode.stopServer)");
        } else {
            tooltip.appendMarkdown("\n\n[Start server](command:rune-vscode.startServer)");
        }

        this.statusBar.tooltip = tooltip;
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

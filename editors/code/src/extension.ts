import * as vscode from "vscode";
import * as lc from "vscode-languageclient/node";
import * as os from "os";

import { log, isValidExecutable, assert, uriExists, setContextValue } from "./util";
import { PersistentState } from "./persistent_state";
import { createClient } from "./client";
import { Config } from "./config";
import { download, fetchRelease } from "./net";

let client: lc.LanguageClient | undefined;
let reload: lc.Disposable | undefined;

const RUNE_PROJECT_CONTEXT_NAME = "inRuneProject";

let TRACE_OUTPUT_CHANNEL: vscode.OutputChannel | null = null;
export function traceOutputChannel() {
	if (!TRACE_OUTPUT_CHANNEL) {
		TRACE_OUTPUT_CHANNEL = vscode.window.createOutputChannel(
			"Rune Language Server Trace"
		);
	}
	return TRACE_OUTPUT_CHANNEL;
}
let OUTPUT_CHANNEL: vscode.OutputChannel | null = null;
export function outputChannel() {
	if (!OUTPUT_CHANNEL) {
		OUTPUT_CHANNEL = vscode.window.createOutputChannel("Rune Language Server");
	}
	return OUTPUT_CHANNEL;
}

export async function activate(
	context: vscode.ExtensionContext
): Promise<lc.LanguageClient> {
	// VS Code doesn't show a notification when an extension fails to activate
	// so we do it ourselves.
	return await tryActivate(context).catch((err) => {
		void vscode.window.showErrorMessage(`Cannot activate rune-vscode: ${err.message}`);
		throw err;
	});
}

async function tryActivate(context: vscode.ExtensionContext): Promise<lc.LanguageClient> {
	const config = new Config(context);
	const state = new PersistentState(context.globalState);
	const serverPath = await bootstrap(context, config, state).catch((err) => {
		let message = "bootstrap error. ";

		message += 'See the logs in "OUTPUT > Rune" (should open automatically). ';
		message += 'To enable verbose logs use { "rune-vscode.trace.extension": true }';

		log.error("Bootstrap error", err);
		throw new Error(message);
	});

	client = createClient(serverPath, config.serverExtraEnv);
	await client.start();

	await setContextValue(RUNE_PROJECT_CONTEXT_NAME, true);

	// Reloading is inspired by @DanTup maneuver: https://github.com/microsoft/vscode/issues/45774#issuecomment-373423895
	reload = vscode.commands.registerCommand("rune-vscode.reload", async () => {
		void vscode.window.showInformationMessage("Reloading Rune Language Server...");
		try {
			await doDeactivate();
		} catch (exception) {
			log.warn("doDeactivate failed", exception);
		}
		await activate(context).catch(log.error);
	});

	return client;
}

export async function deactivate() {
	TRACE_OUTPUT_CHANNEL?.dispose();
	TRACE_OUTPUT_CHANNEL = null;
	OUTPUT_CHANNEL?.dispose();
	OUTPUT_CHANNEL = null;
	reload?.dispose();
	reload = undefined;
	await doDeactivate();
}

async function doDeactivate() {
	await setContextValue(RUNE_PROJECT_CONTEXT_NAME, undefined);
	await client?.stop();
	client = undefined;
}

async function bootstrap(
	context: vscode.ExtensionContext,
	config: Config,
	state: PersistentState
): Promise<string> {
	const path = await getServer(context, config, state);
	if (!path) {
		throw new Error("Rune Language Server is not available.");
	}

	log.info("Using server binary at", path);

	if (!isValidExecutable(path)) {
		if (config.serverPath) {
			throw new Error(`Failed to execute ${path} --version. \`config.server.path\` or \`config.serverPath\` has been set explicitly.\
            Consider removing this config or making a valid server binary available at that path.`);
		} else {
			throw new Error(`Failed to execute ${path} --version`);
		}
	}

	return path;
}

async function getServer(
	context: vscode.ExtensionContext,
	config: Config,
	state: PersistentState
): Promise<string | undefined> {
	// use explicit path from config
	const explicitPath = serverPath(config);
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
	const serverDir = vscode.Uri.joinPath(context.extensionUri, "server");
	const dest = vscode.Uri.joinPath(serverDir, bin);
	const destExists = await uriExists(dest);

	// Only check for updates once every two hours.
	let now = (new Date()).getTime() / 1000;
	let lastCheck = state.lastCheck;
	let timedOut = !lastCheck || (now - lastCheck) > config.updatesCheckInterval;
	log.debug("Check cache timeout", { now, lastCheck, timedOut, timeout: config.updatesCheckInterval });
	if (destExists && !timedOut) {
		return dest.fsPath;
	}

	// fetch new release info
	await state.updateLastCheck(now);
	const release = await fetchRelease("nightly", null);
	const artifact = release.assets.find(artifact => artifact.name === `rune-languageserver-${platform}.gz`);
	assert(!!artifact, `Bad release: ${JSON.stringify(release)}`);

	// no new release
	if (destExists && state.releaseId === artifact.id) {
		return dest.fsPath;
	}

	// ask for update
	if (config.updatesAskBeforeDownload) {
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
	await state.updateReleaseId(release.id);

	return dest.fsPath;
}

function serverPath(config: Config): string | null {
	return process.env.__RUNE_LSP_SERVER_DEBUG ?? config.serverPath;
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
// Parts of these projects have been copied and modified from rust-analyzer under the MIT license: 
// https://github.com/rust-analyzer/rust-analyzer
//
// Copyright of the rust-analyzer developers.

import * as vscode from 'vscode';
import * as lc from 'vscode-languageclient/node';
import * as path from 'path';
import { promises as fs, PathLike } from "fs";
import { log, isValidExecutable, assert, pathExists } from './util'
import { fetchRelease, download } from './net';
import { PersistentState } from './persistent_state';

export async function activate(context: vscode.ExtensionContext) {
    log.info('activating rune language server...');
    await tryActivate(context).catch(err => {
        void vscode.window.showErrorMessage(`Cannot activate rune-languageserver: ${err.message}`);
        throw err;
    });
}

async function tryActivate(context: vscode.ExtensionContext) {
    let platform = detectPlatform();

    if (!platform) {
        return;
    }

    const state = new PersistentState(context.globalState);
    let command = await findCommand(context, state, platform);

    if (!command) {
        log.error('could not find rune language server!');
        return;
    }

    const run: lc.Executable = {
        command: command as string,
        options: {},
    };

    let serverOptions: lc.ServerOptions = {
        run,
        debug: run
    };

    let clientOptions: lc.LanguageClientOptions = {
        documentSelector: [
            {
                scheme: 'file',
                language: 'rune'
            }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**.rn')
        }
    };

    let client = new lc.LanguageClient(
        'Rune',
        'Rune Language Server',
        serverOptions,
        clientOptions
    );

    log.info(`command: ${command}`);
    client.start();
}

/**
 * Find the path to the command to execute.
 */
async function findCommand(
    context: vscode.ExtensionContext,
    state: PersistentState,
    platform: Platform,
): Promise<PathLike | undefined> {
    const exe = `rune-languageserver${platform.ext}`;

    let alternatives = [];

    if (!!process.env.RUNE_DEBUG_FOLDER) {
        alternatives.push(path.join(process.env.RUNE_DEBUG_FOLDER, exe));
    }

    alternatives.push(path.join(process.env.HOME || '~', '.cargo', 'bin', exe));

    for (let p of alternatives) {
        if (await pathExists(p)) {
            return p;
        }
    }

    return await bootstrapServer(context, state, platform);
}

/**
 * Information on the current platform.
 */
interface Platform {
    name: string,
    ext: string,
}

/**
 * Functio used to detect the platform we are on.
 */
function detectPlatform(): Platform | undefined {
    let name: string | undefined;

    if (process.arch === "x64") {
        switch (process.platform) {
        case "win32":
            name = "windows"
            break;
        case "linux":
            name = "linux"
            break;
        case "darwin":
            name = "macos"
            break;
        default:
            break;
        }
    }

    switch (name) {
    case "windows":
        return {name, ext: ".exe"};
    case "linux":
        return {name, ext: ""};
    case "macos":
        return {name, ext: ""};
    default:
        vscode.window.showErrorMessage(
            `Unfortunately we don't support your platform yet.
            You can open an issue about that [here](https://github.com/rune-rs/rune/issues).
            Please include (platform: ${process.platform}, arch: ${process.arch}).`
        );

        return undefined;
    }
}

/** Bootstrap a language server. */
async function bootstrapServer(
    context: vscode.ExtensionContext,
    state: PersistentState,
    platform: Platform,
): Promise<string> {
    const path = await getServer(context, state, platform);

    if (!path) {
        throw new Error("Rune Language Server is not available.");
    }

    log.info("Using server binary at", path);

    if (!isValidExecutable(path)) {
        throw new Error(`Failed to execute: ${path} --version`);
    }

    return path;
}

/** Note: cache time of 2 hours to check for a new release */
const CACHE_TIME = 3600 * 2;

/** Download a language server from GitHub from the "latest" tag. */
async function getServer(
    context: vscode.ExtensionContext,
    state: PersistentState,
    platform: Platform,
): Promise<string | undefined> {
    const bin = `rune-languageserver-${platform.name}${platform.ext}`;
    const dest = path.join(context.globalStoragePath, bin);

    const destExists = await pathExists(dest);

    let now = (new Date()).getTime() / 1000;
    let lastCheck = state.lastCheck;

    let timedOut = !lastCheck || (now - lastCheck) > CACHE_TIME;
    log.debug("Check cache timeout", {now, lastCheck, timedOut, timeout: CACHE_TIME});

    if (destExists && !timedOut) {
        // Only check for updates once every two hours.
        return dest;
    }

    await state.updateLastCheck(now);
    const release = await fetchRelease("latest");

    const artifact = release.assets.find(artifact => artifact.name === `rune-languageserver-${platform.name}.gz`);
    assert(!!artifact, `Bad release: ${JSON.stringify(release)}`);

    if (destExists && state.releaseId == artifact.id) {
        return dest;
    }

    const userResponse = await vscode.window.showInformationMessage(
        `A new version of the Rune Language Server is available (asset id: ${artifact.id}).`,
        "Download now"
    );

    if (userResponse !== "Download now") {
        return dest;
    }

    await fs.unlink(dest).catch(err => {
        if (err.code !== "ENOENT") {
            throw err;
        }
    });

    let globalStorageExists = await pathExists(context.globalStoragePath);

    if (!globalStorageExists) {
        log.debug(`Creating global storage: ${context.globalStoragePath}`);
        await fs.mkdir(context.globalStoragePath);
    }

    await download({
        url: artifact.browser_download_url,
        dest,
        progressTitle: "Downloading Rune Language Server",
        gunzip: true,
        mode: 0o755
    });

    await state.updateReleaseId(release.id);
    return dest;
}

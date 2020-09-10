import * as vscode from 'vscode';
import * as lc from 'vscode-languageclient/node';
import * as path from 'path';

export async function activate(context: vscode.ExtensionContext) {
    await tryActivate(context).catch(err => {
        void vscode.window.showErrorMessage(`Cannot activate rune-languageserver: ${err.message}`);
        throw err;
    });
}

async function tryActivate(_context: vscode.ExtensionContext) {
    let platform = detectPlatform();

    if (!platform) {
        return;
    }

    let command = findCommand(platform);

    if (!command) {
        return;
    }

    const run: lc.Executable = {
        command,
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

    console.log(`command: ${command}`);
    client.start();
}

/**
 * Find the path to the command to execute.
 *
 * @param context The extension context in use.
 * @param platform The detected platform.
 */
function findCommand(platform: Platform): string | undefined {
    if (!!process.env.RUNE_DEBUG_FOLDER) {
        return path.join(process.env.RUNE_DEBUG_FOLDER, `rune-languageserver${platform.ext}`);
    }

    console.debug("Cannot find a command for the Rune Language Server.");
    return undefined;
}

/**
 * Information on the current platform.
 */
interface Platform {
    ext: string,
}

/**
 * Functio used to detect the platform we are on.
 */
function detectPlatform(): Platform | undefined {
    let out: string | undefined;

    if (process.arch === "x64") {
        switch (process.platform) {
        case "win32":
            out = "windows"
            break;
        case "linux":
            out = "linux"
            break;
        case "darwin":
            out = "mac"
            break;
        default:
            break;
        }
    }

    switch (out) {
    case "windows":
        return {ext: ".exe"};
    case "linux":
        return {ext: ""};
    case "mac":
        return {ext: ""};
    default:
        vscode.window.showErrorMessage(
            `Unfortunately we don't support your platform yet.
            You can open an issue about that [here](https://github.com/rune-rs/rune/issues).
            Please include (platform: ${process.platform}, arch: ${process.arch}).`
        );

        return undefined;
    }
}
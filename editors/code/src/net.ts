// Replace with `import fetch from "node-fetch"` once this is fixed in rollup:
// https://github.com/rollup/plugins/issues/491
const fetch = require("node-fetch") as typeof import("node-fetch")["default"];

import * as vscode from "vscode";
import * as stream from "stream";
import * as crypto from "crypto";
import * as fs from "fs";
import * as zlib from "zlib";
import * as util from "util";
import * as path from "path";
import { log, assert } from "./util";

const pipeline = util.promisify(stream.pipeline);

const GITHUB_API_ENDPOINT_URL = "https://api.github.com";
const OWNER = "rune-rs";
const REPO = "rune";

export async function fetchRelease(tag: string): Promise<GithubRelease> {
    const requestUrl = `${GITHUB_API_ENDPOINT_URL}/repos/${OWNER}/${REPO}/releases/tags/${tag}`;
    log.debug("Issuing request for released artifacts metadata to", requestUrl);

    const response = await fetch(requestUrl, { headers: { Accept: "application/vnd.github.v3+json" } });

    if (!response.ok) {
        log.error("Error fetching artifact release info", {
            requestUrl,
            tag,
            response: {
                headers: response.headers,
                status: response.status,
                body: await response.text(),
            }
        });

        throw new Error(
            `Got response ${response.status} when trying to fetch ` +
            `release info for ${tag} release`
        );
    }

    // We skip runtime type checks for simplicity (here we cast from `any` to `GithubRelease`)
    return await response.json();
}

// Information on a single GitHub asset.
export interface GithubAsset {
    // The id of the asset, changes each time its replaced.
    id: number,
    // The name of the asset.
    name: string;
    // eslint-disable-next-line camelcase
    browser_download_url: string;
}

// We omit declaration of tremendous amount of fields that we are not using here
export interface GithubRelease {
    // The name of the release.
    name: string;
    // The id of the release.
    id: number;
    // eslint-disable-next-line camelcase
    published_at: string;
    // collection of assets associated with the release.
    assets: Array<GithubAsset>;
}

interface DownloadOpts {
    progressTitle: string;
    url: string;
    dest: string;
    mode?: number;
    gunzip?: boolean;
}

export async function download(opts: DownloadOpts) {
    // Put artifact into a temporary file (in the same dir for simplicity)
    // to prevent partially downloaded files when user kills vscode
    const dest = path.parse(opts.dest);
    const randomHex = crypto.randomBytes(5).toString("hex");
    const tempFile = path.join(dest.dir, `${dest.name}${randomHex}`);

    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            cancellable: false,
            title: opts.progressTitle
        },
        async (progress, _cancellationToken) => {
            let lastPercentage = 0;
            await downloadFile(opts.url, tempFile, opts.mode, !!opts.gunzip, (readBytes, totalBytes) => {
                const newPercentage = (readBytes / totalBytes) * 100;
                progress.report({
                    message: newPercentage.toFixed(0) + "%",
                    increment: newPercentage - lastPercentage
                });

                lastPercentage = newPercentage;
            });
        }
    );

    await fs.promises.rename(tempFile, opts.dest);
}

async function downloadFile(
    url: string,
    destFilePath: fs.PathLike,
    mode: number | undefined,
    gunzip: boolean,
    onProgress: (readBytes: number, totalBytes: number) => void
): Promise<void> {
    const res = await fetch(url);

    if (!res.ok) {
        log.error("Error", res.status, "while downloading file from", url);
        log.error({ body: await res.text(), headers: res.headers });

        throw new Error(`Got response ${res.status} when trying to download a file.`);
    }

    const totalBytes = Number(res.headers.get('content-length'));
    assert(!Number.isNaN(totalBytes), "Sanity check of content-length protocol");

    log.debug("Downloading file of", totalBytes, "bytes size from", url, "to", destFilePath);

    let readBytes = 0;
    res.body.on("data", (chunk: Buffer) => {
        readBytes += chunk.length;
        onProgress(readBytes, totalBytes);
    });

    const destFileStream = fs.createWriteStream(destFilePath, { mode });
    const srcStream = gunzip ? res.body.pipe(zlib.createGunzip()) : res.body;

    await pipeline(srcStream, destFileStream);

    // Don't apply the workaround in fixed versions of nodejs, since the process
    // freezes on them, the process waits for no-longer emitted `close` event.
    // The fix was applied in commit 7eed9d6bcc in v13.11.0
    // See the nodejs changelog:
    // https://github.com/nodejs/node/blob/master/doc/changelogs/CHANGELOG_V13.md
    const [, major, minor] = /v(\d+)\.(\d+)\.(\d+)/.exec(process.version)!;
    if (+major > 13 || (+major === 13 && +minor >= 11)) return;

    await new Promise<void>(resolve => {
        destFileStream.on("close", resolve);
        destFileStream.destroy();
        // This workaround is awaiting to be removed when vscode moves to newer nodejs version:
        // https://github.com/rust-analyzer/rust-analyzer/issues/3167
    });
}
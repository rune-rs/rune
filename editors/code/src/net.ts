import * as vscode from "vscode";
import * as crypto from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";
import * as zlib from "node:zlib";
import { Readable, Transform } from "node:stream";
import { pipeline } from "node:stream/promises";
import type { ReadableStream as WebReadableStream } from "node:stream/web";
import { log } from "./util";

const GITHUB_API_ENDPOINT_URL = "https://api.github.com";
const OWNER = "rune-rs";
const REPO = "rune";

export async function fetchRelease(
	releaseTag: string,
	githubToken?: string | null,
): Promise<GithubRelease> {
	const apiEndpointPath = `/repos/${OWNER}/${REPO}/releases/tags/${releaseTag}`;
	const requestUrl = GITHUB_API_ENDPOINT_URL + apiEndpointPath;

	log.debug("Issuing request for released artifacts metadata to", requestUrl);

	const headers: Record<string, string> = {
		Accept: "application/vnd.github.v3+json",
	};
	if (githubToken) {
		headers.Authorization = `token ${githubToken}`;
	}

	const response = await fetch(requestUrl, { headers });

	if (!response.ok) {
		log.error("Error fetching artifact release info", {
			requestUrl,
			releaseTag,
			response: {
				headers: Object.fromEntries(response.headers.entries()),
				status: response.status,
				body: await response.text(),
			},
		});

		throw new Error(
			`Got response ${response.status} when trying to fetch ` +
				`release info for ${releaseTag} release`,
		);
	}

	return (await response.json()) as GithubRelease;
}

export interface GithubRelease {
	name: string;
	id: number;
	published_at: string;
	assets: Array<{
		name: string;
		id: number;
		browser_download_url: string;
	}>;
}

interface DownloadOpts {
	progressTitle: string;
	url: string;
	dest: vscode.Uri;
	mode?: number;
	gunzip?: boolean;
}

export async function download(opts: DownloadOpts): Promise<void> {
	// Put artifact into a temporary file (in the same dir for simplicity)
	// to prevent partially downloaded files when user kills vscode.
	// This also avoids overwriting running executables.
	const randomHex = crypto.randomBytes(5).toString("hex");
	const rawDest = path.parse(opts.dest.fsPath);
	const serverDir = vscode.Uri.file(rawDest.dir);
	const oldServerPath = vscode.Uri.joinPath(
		serverDir,
		`${rawDest.name}-stale-${randomHex}${rawDest.ext}`,
	);
	const tempFilePath = vscode.Uri.joinPath(serverDir, `${rawDest.name}${randomHex}`);

	await vscode.window.withProgress(
		{
			location: vscode.ProgressLocation.Notification,
			cancellable: false,
			title: opts.progressTitle,
		},
		async (progress) => {
			let lastPercentage = 0;
			await downloadFile(
				opts.url,
				tempFilePath,
				opts.mode,
				Boolean(opts.gunzip),
				(readBytes, totalBytes) => {
					if (totalBytes <= 0) {
						return;
					}
					const newPercentage = Math.round((readBytes / totalBytes) * 100);
					if (newPercentage !== lastPercentage) {
						progress.report({
							message: `${newPercentage.toFixed(0)}%`,
							increment: newPercentage - lastPercentage,
						});
						lastPercentage = newPercentage;
					}
				},
			);
		},
	);

	// Try to rename a running server to avoid EPERM on Windows.
	try {
		await vscode.workspace.fs.rename(opts.dest, oldServerPath, { overwrite: true });
		log.info(`Renamed old server binary ${opts.dest.fsPath} to ${oldServerPath.fsPath}`);
	} catch (err) {
		const isNotFound =
			err instanceof vscode.FileSystemError &&
			(err.code === "FileNotFound" ||
				err.code === "EntryNotFound" ||
				err.name.includes("EntryNotFound"));
		if (!isNotFound) {
			log.error(`Cannot rename existing server instance: ${err}`);
		}
	}

	try {
		await vscode.workspace.fs.rename(tempFilePath, opts.dest, { overwrite: true });
	} catch (err) {
		log.error(`Cannot update server binary: ${err}`);
	}

	// Now try to remove any stale server binaries.
	try {
		const entries = await vscode.workspace.fs.readDirectory(serverDir);
		for (const [entry] of entries) {
			if (entry.includes(`${rawDest.name}-stale-`)) {
				const uri = vscode.Uri.joinPath(serverDir, entry);
				try {
					await vscode.workspace.fs.delete(uri);
					log.info(`Removed old server binary ${uri.fsPath}`);
				} catch {
					log.error(`Unable to remove old server binary ${uri.fsPath}`);
				}
			}
		}
	} catch {
		log.error(`Unable to enumerate contents of ${serverDir.fsPath}`);
	}
}

async function downloadFile(
	url: string,
	destFilePath: vscode.Uri,
	mode: number | undefined,
	gunzip: boolean,
	onProgress: (readBytes: number, totalBytes: number) => void,
): Promise<void> {
	const res = await fetch(url);

	if (!res.ok) {
		log.error("Error", res.status, "while downloading file from", url);
		log.error({
			body: await res.text(),
			headers: Object.fromEntries(res.headers.entries()),
		});

		throw new Error(`Got response ${res.status} when trying to download a file.`);
	}

	if (!res.body) {
		log.error("Empty body while downloading file from", url);
		log.error({ headers: Object.fromEntries(res.headers.entries()) });
		throw new Error("Got empty body when trying to download a file.");
	}

	const contentLength = res.headers.get("content-length");
	const totalBytes = contentLength ? Number(contentLength) : 0;

	log.debug("Downloading file of", totalBytes, "bytes size from", url, "to", destFilePath.fsPath);

	let readBytes = 0;
	const progressStream = new Transform({
		transform(chunk: Buffer, _encoding, callback) {
			readBytes += chunk.length;
			onProgress(readBytes, totalBytes);
			callback(null, chunk);
		},
	});

	const nodeBody = Readable.fromWeb(res.body as unknown as WebReadableStream<Uint8Array>);
	const destFileStream = fs.createWriteStream(destFilePath.fsPath, { mode });

	if (gunzip) {
		await pipeline(nodeBody, progressStream, zlib.createGunzip(), destFileStream);
	} else {
		await pipeline(nodeBody, progressStream, destFileStream);
	}
}

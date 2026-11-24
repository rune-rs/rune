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
	zipEntry?: string;
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

	try {
		await vscode.window.withProgress(
			{
				location: vscode.ProgressLocation.Notification,
				cancellable: true,
				title: opts.progressTitle,
			},
			async (progress, cancellationToken) => {
				const controller = new AbortController();
				const cancelListener = cancellationToken.onCancellationRequested(() => {
					controller.abort();
				});

				let lastPercentage = 0;
				try {
					await downloadFile(
						opts.url,
						tempFilePath,
						opts.mode,
						Boolean(opts.gunzip),
						opts.zipEntry,
						controller.signal,
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
				} catch (err) {
					if (cancellationToken.isCancellationRequested) {
						throw new Error("Download cancelled by user.");
					}
					throw err;
				} finally {
					cancelListener.dispose();
				}
			},
		);
	} catch (err) {
		await deleteQuietly(tempFilePath);
		throw err;
	}

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
		await deleteQuietly(tempFilePath);
		throw new Error(`Cannot update server binary: ${err}`);
	}

	// Ensure executable permissions are set after rename (vscode.workspace.fs.rename
	// and process umask can strip mode bits on POSIX systems).
	if (opts.mode !== undefined && process.platform !== "win32") {
		await fs.promises.chmod(opts.dest.fsPath, opts.mode);
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

async function deleteQuietly(uri: vscode.Uri): Promise<void> {
	try {
		await vscode.workspace.fs.delete(uri);
	} catch {
		// Ignore cleanup failures
	}
}

async function downloadFile(
	url: string,
	destFilePath: vscode.Uri,
	mode: number | undefined,
	gunzip: boolean,
	zipEntry: string | undefined,
	signal: AbortSignal,
	onProgress: (readBytes: number, totalBytes: number) => void,
): Promise<void> {
	const res = await fetch(url, { signal });

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
	const nodeBody = Readable.fromWeb(res.body as unknown as WebReadableStream<Uint8Array>);

	if (zipEntry) {
		const chunks: Buffer[] = [];
		for await (const chunk of nodeBody) {
			const buf = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk as Uint8Array);
			readBytes += buf.length;
			onProgress(readBytes, totalBytes);
			chunks.push(buf);
		}
		const archive = Buffer.concat(chunks);
		const extracted = extractZipEntry(archive, zipEntry);
		await fs.promises.writeFile(destFilePath.fsPath, extracted, { mode });
		return;
	}

	const progressStream = new Transform({
		transform(chunk: Buffer, _encoding, callback) {
			readBytes += chunk.length;
			onProgress(readBytes, totalBytes);
			callback(null, chunk);
		},
	});

	const destFileStream = fs.createWriteStream(destFilePath.fsPath, { mode });

	if (gunzip) {
		await pipeline(nodeBody, progressStream, zlib.createGunzip(), destFileStream, { signal });
	} else {
		await pipeline(nodeBody, progressStream, destFileStream, { signal });
	}
}

/**
 * Extract a single named file from a ZIP archive buffer (supports Stored and Deflate).
 */
function extractZipEntry(buf: Buffer, entryName: string): Buffer {
	const minEocdOffset = Math.max(0, buf.length - 65557);
	let eocdOffset = -1;
	for (let i = buf.length - 22; i >= minEocdOffset; i--) {
		if (buf.readUInt32LE(i) === 0x06054b50) {
			eocdOffset = i;
			break;
		}
	}
	if (eocdOffset === -1) {
		throw new Error("Invalid ZIP archive: End of Central Directory not found.");
	}

	const entryCount = buf.readUInt16LE(eocdOffset + 10);
	let pos = buf.readUInt32LE(eocdOffset + 16);

	for (let i = 0; i < entryCount; i++) {
		if (buf.readUInt32LE(pos) !== 0x02014b50) {
			throw new Error("Invalid ZIP archive: malformed Central Directory header.");
		}
		const method = buf.readUInt16LE(pos + 10);
		const compressedSize = buf.readUInt32LE(pos + 20);
		const nameLen = buf.readUInt16LE(pos + 28);
		const extraLen = buf.readUInt16LE(pos + 30);
		const commentLen = buf.readUInt16LE(pos + 32);
		const localHeaderOffset = buf.readUInt32LE(pos + 42);
		const name = buf.toString("utf8", pos + 46, pos + 46 + nameLen);

		if (name === entryName || path.posix.basename(name) === entryName) {
			if (buf.readUInt32LE(localHeaderOffset) !== 0x04034b50) {
				throw new Error("Invalid ZIP archive: malformed Local File header.");
			}
			const localNameLen = buf.readUInt16LE(localHeaderOffset + 26);
			const localExtraLen = buf.readUInt16LE(localHeaderOffset + 28);
			const dataStart = localHeaderOffset + 30 + localNameLen + localExtraLen;
			const compressed = buf.subarray(dataStart, dataStart + compressedSize);

			if (method === 0) {
				return compressed;
			}
			if (method === 8) {
				return zlib.inflateRawSync(compressed);
			}
			throw new Error(`Unsupported ZIP compression method: ${method}`);
		}

		pos += 46 + nameLen + extraLen + commentLen;
	}

	throw new Error(`Entry "${entryName}" not found in ZIP archive.`);
}

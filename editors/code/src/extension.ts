import * as vscode from "vscode";
import { Ctx } from "./ctx";
import { log } from "./util";

let ctx: Ctx | undefined;
let traceChannel: vscode.LogOutputChannel | undefined;
let serverChannel: vscode.LogOutputChannel | undefined;

export function traceOutputChannel(): vscode.LogOutputChannel {
	if (!traceChannel) {
		traceChannel = vscode.window.createOutputChannel("Rune Language Server Trace", { log: true });
	}
	return traceChannel;
}

export function outputChannel(): vscode.LogOutputChannel {
	if (!serverChannel) {
		serverChannel = vscode.window.createOutputChannel("Rune Language Server", { log: true });
	}
	return serverChannel;
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
	if (!ctx) {
		ctx = new Ctx(context);
	}

	await ctx.activate();
}

export async function deactivate(): Promise<void> {
	await ctx?.dispose();
	ctx = undefined;
	traceChannel?.dispose();
	traceChannel = undefined;
	serverChannel?.dispose();
	serverChannel = undefined;
	log.dispose();
}

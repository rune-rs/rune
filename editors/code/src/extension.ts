import * as vscode from "vscode";
import { Ctx } from "./ctx";

let ctx: Ctx | undefined;

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

export async function activate(context: vscode.ExtensionContext) {
	if (!ctx) {
		ctx = new Ctx(context);
	}

	ctx.activate();
}

export async function deactivate() {
	TRACE_OUTPUT_CHANNEL?.dispose();
	TRACE_OUTPUT_CHANNEL = null;
	OUTPUT_CHANNEL?.dispose();
	OUTPUT_CHANNEL = null;
	ctx?.dispose();
	ctx = undefined;
}

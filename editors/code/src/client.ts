import * as lc from "vscode-languageclient/node";

import { type Env, substituteVariablesInEnv } from "./config";
import { outputChannel, traceOutputChannel } from "./extension";
import { log } from "./util";

export function createClient(
	serverPath: string,
	extraEnv: Env,
	args?: string[],
): lc.LanguageClient {
	const newEnv = substituteVariablesInEnv({ ...process.env, ...extraEnv });
	log.debug("newEnv", newEnv);

	const run: lc.Executable = {
		command: serverPath,
		options: { env: newEnv },
		args,
	};

	const serverOptions: lc.ServerOptions = {
		run,
		debug: run,
	};

	const clientOptions: lc.LanguageClientOptions = {
		documentSelector: [
			{ scheme: "file", language: "rune" },
			{ pattern: "**/Rune.toml", language: "toml" },
		],
		traceOutputChannel: traceOutputChannel(),
		outputChannel: outputChannel(),
	};

	return new lc.LanguageClient(
		"rune-languageserver",
		"Rune Language Server",
		serverOptions,
		clientOptions,
	);
}

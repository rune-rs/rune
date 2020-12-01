import * as vscode from "vscode";


export interface ISettings {
    binaryPath: string,
}

export function load(myPluginId: string): ISettings {
    let configuration = vscode.workspace.getConfiguration(myPluginId);
    return {
        binaryPath: configuration.get<string>("binaryPath", "")
    }
}
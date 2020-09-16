import * as vscode from 'vscode';
import { log } from './util';

export class PersistentState {
    constructor(private readonly globalState: vscode.Memento) {
        const { lastCheck, releaseId } = this;
        log.info("PersistentState:", { lastCheck, releaseId });
    }

    get lastCheck(): number | undefined {
        return this.globalState.get("lastCheck");
    }

    async updateLastCheck(value: number) {
        await this.globalState.update("lastCheck", value);
    }

    get releaseId(): number | undefined {
        return this.globalState.get("releaseId");
    }

    async updateReleaseId(value: number) {
        await this.globalState.update("releaseId", value);
    }
}
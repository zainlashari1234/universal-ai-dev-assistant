import * as vscode from 'vscode';

export class StatusBarManager implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;
    private progressTimeout?: NodeJS.Timeout;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            100
        );
        this.statusBarItem.text = '$(robot) UAIDA';
        this.statusBarItem.tooltip = 'Universal AI Development Assistant';
        this.statusBarItem.command = 'uaida.plan';
        this.statusBarItem.show();
    }

    showProgress(message: string) {
        this.statusBarItem.text = `$(loading~spin) ${message}`;
        this.statusBarItem.tooltip = message;
        this.statusBarItem.backgroundColor = undefined;
        
        // Auto-hide after 30 seconds
        if (this.progressTimeout) {
            clearTimeout(this.progressTimeout);
        }
        this.progressTimeout = setTimeout(() => {
            this.reset();
        }, 30000);
    }

    showSuccess(message: string) {
        this.statusBarItem.text = `$(check) ${message}`;
        this.statusBarItem.tooltip = message;
        this.statusBarItem.backgroundColor = undefined;
        
        // Auto-reset after 5 seconds
        setTimeout(() => {
            this.reset();
        }, 5000);
    }

    showError(message: string) {
        this.statusBarItem.text = `$(error) ${message}`;
        this.statusBarItem.tooltip = message;
        this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        
        // Auto-reset after 10 seconds
        setTimeout(() => {
            this.reset();
        }, 10000);
    }

    showWarning(message: string) {
        this.statusBarItem.text = `$(warning) ${message}`;
        this.statusBarItem.tooltip = message;
        this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        
        // Auto-reset after 8 seconds
        setTimeout(() => {
            this.reset();
        }, 8000);
    }

    reset() {
        if (this.progressTimeout) {
            clearTimeout(this.progressTimeout);
            this.progressTimeout = undefined;
        }
        
        this.statusBarItem.text = '$(robot) UAIDA';
        this.statusBarItem.tooltip = 'Universal AI Development Assistant - Click to create plan';
        this.statusBarItem.backgroundColor = undefined;
        this.statusBarItem.command = 'uaida.plan';
    }

    hide() {
        this.statusBarItem.hide();
    }

    show() {
        this.statusBarItem.show();
    }

    dispose() {
        if (this.progressTimeout) {
            clearTimeout(this.progressTimeout);
        }
        this.statusBarItem.dispose();
    }
}
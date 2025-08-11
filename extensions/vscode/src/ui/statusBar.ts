import * as vscode from 'vscode';

export class StatusBarManager implements vscode.Disposable {
    private statusBarItem: vscode.StatusBarItem;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'uaida.analyze';
        this.statusBarItem.show();
        this.setDisconnected();
    }

    setConnected(connected: boolean) {
        if (connected) {
            this.statusBarItem.text = '$(check) UAIDA';
            this.statusBarItem.tooltip = 'Universal AI Dev Assistant - Connected';
            this.statusBarItem.backgroundColor = undefined;
        } else {
            this.setDisconnected();
        }
    }

    setDisconnected() {
        this.statusBarItem.text = '$(x) UAIDA';
        this.statusBarItem.tooltip = 'Universal AI Dev Assistant - Disconnected';
        this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
    }

    setLoading(loading: boolean) {
        if (loading) {
            this.statusBarItem.text = '$(sync~spin) UAIDA';
            this.statusBarItem.tooltip = 'Universal AI Dev Assistant - Processing...';
        } else {
            this.setConnected(true);
        }
    }

    dispose() {
        this.statusBarItem.dispose();
    }
}
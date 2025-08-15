import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    console.log('UAIDA extension is now active!');

    // Register commands
    let planCommand = vscode.commands.registerCommand('uaida.plan', () => {
        vscode.window.showInformationMessage('UAIDA: Create Plan command executed!');
    });

    let patchCommand = vscode.commands.registerCommand('uaida.patch', () => {
        vscode.window.showInformationMessage('UAIDA: Apply Patch command executed!');
    });

    let testCommand = vscode.commands.registerCommand('uaida.test', () => {
        vscode.window.showInformationMessage('UAIDA: Run Tests command executed!');
    });

    let reviewCommand = vscode.commands.registerCommand('uaida.review', () => {
        vscode.window.showInformationMessage('UAIDA: Code Review command executed!');
    });

    let rollbackCommand = vscode.commands.registerCommand('uaida.rollback', () => {
        vscode.window.showInformationMessage('UAIDA: Rollback command executed!');
    });

    context.subscriptions.push(planCommand, patchCommand, testCommand, reviewCommand, rollbackCommand);
}

export function deactivate() {
    console.log('UAIDA extension is now deactivated!');
}
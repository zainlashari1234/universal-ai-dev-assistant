import * as vscode from 'vscode';
import { AICompletionProvider } from './providers/completionProvider';
import { AICodeLensProvider } from './providers/codeLensProvider';
import { AIHoverProvider } from './providers/hoverProvider';
import { UAIDAClient } from './client/uaidaClient';
import { StatusBarManager } from './ui/statusBar';

let client: UAIDAClient;
let statusBar: StatusBarManager;

export function activate(context: vscode.ExtensionContext) {
    console.log('Universal AI Development Assistant is now active!');

    // Initialize client
    client = new UAIDAClient();
    statusBar = new StatusBarManager();

    // Register completion provider
    const completionProvider = new AICompletionProvider(client);
    const completionDisposable = vscode.languages.registerCompletionItemProvider(
        [
            { scheme: 'file', language: 'python' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'go' },
            { scheme: 'file', language: 'java' },
            { scheme: 'file', language: 'cpp' },
            { scheme: 'file', language: 'c' }
        ],
        completionProvider,
        '.' // Trigger on dot
    );

    // Register code lens provider
    const codeLensProvider = new AICodeLensProvider(client);
    const codeLensDisposable = vscode.languages.registerCodeLensProvider(
        '*',
        codeLensProvider
    );

    // Register hover provider
    const hoverProvider = new AIHoverProvider(client);
    const hoverDisposable = vscode.languages.registerHoverProvider(
        '*',
        hoverProvider
    );

    // Register commands
    const commands = [
        vscode.commands.registerCommand('uaida.complete', async () => {
            await handleManualCompletion();
        }),
        vscode.commands.registerCommand('uaida.analyze', async () => {
            await handleCodeAnalysis();
        }),
        vscode.commands.registerCommand('uaida.refactor', async () => {
            await handleRefactoring();
        }),
        vscode.commands.registerCommand('uaida.generateDocs', async () => {
            await handleDocGeneration();
        }),
        vscode.commands.registerCommand('uaida.generateTests', async () => {
            await handleTestGeneration();
        })
    ];

    // Add all disposables to context
    context.subscriptions.push(
        completionDisposable,
        codeLensDisposable,
        hoverDisposable,
        statusBar,
        ...commands
    );

    // Check server connection
    checkServerConnection();

    // Setup configuration change listener
    vscode.workspace.onDidChangeConfiguration(event => {
        if (event.affectsConfiguration('uaida')) {
            client.updateConfiguration();
        }
    });
}

async function handleManualCompletion() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    const position = editor.selection.active;
    const code = document.getText();
    const offset = document.offsetAt(position);

    try {
        statusBar.setLoading(true);
        const suggestions = await client.getCompletion({
            code,
            language: document.languageId,
            cursor_position: offset
        });

        if (suggestions.length > 0) {
            const items = suggestions.map(suggestion => ({
                label: suggestion,
                insertText: suggestion
            }));

            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select a completion'
            });

            if (selected) {
                await editor.edit(editBuilder => {
                    editBuilder.insert(position, selected.insertText);
                });
            }
        } else {
            vscode.window.showInformationMessage('No suggestions available');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Completion failed: ${error}`);
    } finally {
        statusBar.setLoading(false);
    }
}

async function handleCodeAnalysis() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showErrorMessage('No active editor');
        return;
    }

    const document = editor.document;
    const code = document.getText();

    try {
        statusBar.setLoading(true);
        const analysis = await client.analyzeCode({
            code,
            language: document.languageId,
            cursor_position: 0
        });

        // Show analysis in a webview
        const panel = vscode.window.createWebviewPanel(
            'uaidaAnalysis',
            'Code Analysis',
            vscode.ViewColumn.Beside,
            { enableScripts: true }
        );

        panel.webview.html = generateAnalysisHTML(analysis);
    } catch (error) {
        vscode.window.showErrorMessage(`Analysis failed: ${error}`);
    } finally {
        statusBar.setLoading(false);
    }
}

async function handleRefactoring() {
    vscode.window.showInformationMessage('Refactoring suggestions coming soon!');
}

async function handleDocGeneration() {
    vscode.window.showInformationMessage('Documentation generation coming soon!');
}

async function handleTestGeneration() {
    vscode.window.showInformationMessage('Test generation coming soon!');
}

async function checkServerConnection() {
    try {
        const isConnected = await client.checkHealth();
        if (isConnected) {
            statusBar.setConnected(true);
            vscode.window.showInformationMessage('UAIDA: Connected to AI server');
        } else {
            statusBar.setConnected(false);
            vscode.window.showWarningMessage('UAIDA: Cannot connect to AI server');
        }
    } catch (error) {
        statusBar.setConnected(false);
        vscode.window.showErrorMessage(`UAIDA: Server connection failed: ${error}`);
    }
}

function generateAnalysisHTML(analysis: any): string {
    return `
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <title>Code Analysis</title>
        <style>
            body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
            .section { margin: 20px 0; }
            .issue { background: #fff3cd; padding: 10px; margin: 5px 0; border-radius: 4px; }
            .suggestion { background: #d1ecf1; padding: 10px; margin: 5px 0; border-radius: 4px; }
            .metric { display: inline-block; margin: 10px; padding: 10px; background: #f8f9fa; border-radius: 4px; }
        </style>
    </head>
    <body>
        <h1>Code Analysis Results</h1>
        
        <div class="section">
            <h2>Issues</h2>
            ${analysis.issues?.map((issue: any) => `
                <div class="issue">
                    <strong>${issue.type}</strong>: ${issue.message}
                    ${issue.line ? `(Line ${issue.line})` : ''}
                </div>
            `).join('') || '<p>No issues found</p>'}
        </div>

        <div class="section">
            <h2>Suggestions</h2>
            ${analysis.suggestions?.map((suggestion: any) => `
                <div class="suggestion">
                    <strong>${suggestion.type}</strong>: ${suggestion.message}
                    ${suggestion.line ? `(Line ${suggestion.line})` : ''}
                </div>
            `).join('') || '<p>No suggestions available</p>'}
        </div>

        <div class="section">
            <h2>Metrics</h2>
            <div class="metric">
                <strong>Complexity:</strong> ${analysis.complexity?.cyclomatic || 'N/A'}
            </div>
            <div class="metric">
                <strong>Maintainability:</strong> ${analysis.complexity?.maintainability_index || 'N/A'}
            </div>
            <div class="metric">
                <strong>Test Coverage:</strong> ${analysis.test_coverage || 'N/A'}%
            </div>
        </div>
    </body>
    </html>
    `;
}

export function deactivate() {
    if (client) {
        client.dispose();
    }
}
import * as vscode from 'vscode';
import axios from 'axios';

interface CompletionRequest {
    prompt: string;
    language: string;
    max_tokens?: number;
    temperature?: number;
    provider?: string;
}

interface CompletionResponse {
    suggestions: Array<{
        text: string;
        confidence: number;
        provider: string;
    }>;
}

class UAIDAClient {
    private baseUrl: string;
    private apiKey: string;

    constructor() {
        const config = vscode.workspace.getConfiguration('uaida');
        this.baseUrl = config.get('apiUrl', 'http://localhost:8080');
        this.apiKey = config.get('apiKey', '');
    }

    async getCompletion(request: CompletionRequest): Promise<CompletionResponse> {
        try {
            const response = await axios.post(`${this.baseUrl}/api/v1/complete`, request, {
                headers: {
                    'Content-Type': 'application/json',
                    ...(this.apiKey && { 'Authorization': `Bearer ${this.apiKey}` })
                },
                timeout: 10000
            });
            return response.data;
        } catch (error) {
            throw new Error(`UAIDA API error: ${error}`);
        }
    }

    async analyzeCode(code: string, language: string): Promise<any> {
        try {
            const response = await axios.post(`${this.baseUrl}/api/v1/analyze`, {
                code,
                language
            }, {
                headers: {
                    'Content-Type': 'application/json',
                    ...(this.apiKey && { 'Authorization': `Bearer ${this.apiKey}` })
                },
                timeout: 15000
            });
            return response.data;
        } catch (error) {
            throw new Error(`UAIDA Analysis error: ${error}`);
        }
    }
}

export function activate(context: vscode.ExtensionContext) {
    console.log('Universal AI Development Assistant is now active!');

    const client = new UAIDAClient();

    // Code Completion Command
    const completeCommand = vscode.commands.registerCommand('uaida.complete', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor found');
            return;
        }

        const document = editor.document;
        const position = editor.selection.active;
        const textBeforeCursor = document.getText(new vscode.Range(new vscode.Position(0, 0), position));

        if (textBeforeCursor.trim().length < 10) {
            vscode.window.showInformationMessage('Please write more code for better suggestions');
            return;
        }

        try {
            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: "Getting AI suggestions...",
                cancellable: false
            }, async () => {
                const config = vscode.workspace.getConfiguration('uaida');
                const response = await client.getCompletion({
                    prompt: textBeforeCursor,
                    language: document.languageId,
                    max_tokens: config.get('maxTokens', 100),
                    temperature: config.get('temperature', 0.7),
                    provider: config.get('defaultProvider', 'openai')
                });

                if (response.suggestions && response.suggestions.length > 0) {
                    const suggestion = response.suggestions[0];
                    const edit = new vscode.WorkspaceEdit();
                    edit.insert(document.uri, position, suggestion.text);
                    await vscode.workspace.applyEdit(edit);
                    
                    vscode.window.showInformationMessage(
                        `Code completed using ${suggestion.provider} (${Math.round(suggestion.confidence * 100)}% confidence)`
                    );
                } else {
                    vscode.window.showWarningMessage('No suggestions available');
                }
            });
        } catch (error) {
            vscode.window.showErrorMessage(`Completion failed: ${error}`);
        }
    });

    // Code Analysis Command
    const analyzeCommand = vscode.commands.registerCommand('uaida.analyze', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor found');
            return;
        }

        const code = editor.document.getText();
        const language = editor.document.languageId;

        try {
            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: "Analyzing code...",
                cancellable: false
            }, async () => {
                const analysis = await client.analyzeCode(code, language);
                
                // Create and show analysis results
                const doc = await vscode.workspace.openTextDocument({
                    content: formatAnalysisResults(analysis),
                    language: 'markdown'
                });
                await vscode.window.showTextDocument(doc);
            });
        } catch (error) {
            vscode.window.showErrorMessage(`Analysis failed: ${error}`);
        }
    });

    // Chat Command
    const chatCommand = vscode.commands.registerCommand('uaida.chat', async () => {
        const input = await vscode.window.showInputBox({
            prompt: 'Ask AI about your code',
            placeHolder: 'e.g., How can I optimize this function?'
        });

        if (input) {
            vscode.window.showInformationMessage(`AI Chat: ${input} (Feature coming soon!)`);
        }
    });

    // Explain Code Command
    const explainCommand = vscode.commands.registerCommand('uaida.explain', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor found');
            return;
        }

        const selection = editor.selection;
        const code = selection.isEmpty ? editor.document.getText() : editor.document.getText(selection);

        vscode.window.showInformationMessage(`Explaining code: ${code.substring(0, 50)}... (Feature coming soon!)`);
    });

    // Refactor Command
    const refactorCommand = vscode.commands.registerCommand('uaida.refactor', async () => {
        vscode.window.showInformationMessage('AI Refactoring feature coming soon!');
    });

    // Generate Tests Command
    const generateTestsCommand = vscode.commands.registerCommand('uaida.generateTests', async () => {
        vscode.window.showInformationMessage('AI Test Generation feature coming soon!');
    });

    // Register all commands
    context.subscriptions.push(
        completeCommand,
        analyzeCommand,
        chatCommand,
        explainCommand,
        refactorCommand,
        generateTestsCommand
    );

    // Status bar item
    const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.text = '$(robot) UAIDA';
    statusBarItem.tooltip = 'Universal AI Development Assistant';
    statusBarItem.command = 'uaida.complete';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    vscode.window.showInformationMessage('Universal AI Development Assistant activated! Use Ctrl+Shift+Space for completions.');
}

function formatAnalysisResults(analysis: any): string {
    return `# Code Analysis Results

## Security Issues
${analysis.security_issues?.map((issue: any) => 
    `- **${issue.severity?.toUpperCase()}**: ${issue.message} (Line ${issue.line || 'N/A'})`
).join('\n') || 'No security issues found'}

## Performance Suggestions
${analysis.performance_suggestions?.map((suggestion: any) => 
    `- ${suggestion.message} (Line ${suggestion.line || 'N/A'})`
).join('\n') || 'No performance suggestions'}

## Code Quality Score: ${analysis.code_quality?.score || 'N/A'}/100

### Quality Issues
${analysis.code_quality?.issues?.map((issue: any) => 
    `- **${issue.severity?.toUpperCase()}**: ${issue.message} (Line ${issue.line || 'N/A'})`
).join('\n') || 'No quality issues found'}
`;
}

export function deactivate() {
    console.log('Universal AI Development Assistant deactivated');
}
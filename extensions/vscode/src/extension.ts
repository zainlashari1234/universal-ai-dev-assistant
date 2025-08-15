import * as vscode from 'vscode';
import { UAIDAClient } from './client';
import { ChatPanel } from './chatPanel';
import { StatusBarManager } from './statusBar';
import { DiagnosticsProvider } from './diagnostics';
import { CompletionProvider } from './completion';

let client: UAIDAClient;
let chatPanel: ChatPanel;
let statusBar: StatusBarManager;
let diagnosticsProvider: DiagnosticsProvider;
let completionProvider: CompletionProvider;

export function activate(context: vscode.ExtensionContext) {
    console.log('🚀 UAIDA extension is now active!');

    // Initialize client
    const config = vscode.workspace.getConfiguration('uaida');
    const serverUrl = config.get<string>('serverUrl', 'http://localhost:8080');
    client = new UAIDAClient(serverUrl);

    // Initialize components
    statusBar = new StatusBarManager(client);
    chatPanel = new ChatPanel(context, client);
    diagnosticsProvider = new DiagnosticsProvider(client);
    completionProvider = new CompletionProvider(client);

    // Register commands
    registerCommands(context);

    // Register providers
    registerProviders(context);

    // Start background services
    startBackgroundServices();

    console.log('✅ UAIDA extension fully initialized');
}

function registerCommands(context: vscode.ExtensionContext) {
    // Code completion command
    const completeCommand = vscode.commands.registerCommand('uaida.complete', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        const text = selection.isEmpty ? 
            editor.document.getText() : 
            editor.document.getText(selection);

        if (!text.trim()) {
            vscode.window.showErrorMessage('No code selected');
            return;
        }

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "🤖 Generating code completion...",
            cancellable: true
        }, async (progress, token) => {
            try {
                const language = editor.document.languageId;
                const completion = await client.complete({
                    prompt: text,
                    language,
                    max_tokens: vscode.workspace.getConfiguration('uaida').get('maxTokens', 1000),
                    temperature: vscode.workspace.getConfiguration('uaida').get('temperature', 0.7)
                });

                if (token.isCancellationRequested) return;

                // Insert completion
                const position = selection.isEmpty ? editor.selection.active : selection.end;
                await editor.edit(editBuilder => {
                    editBuilder.insert(position, '\n' + completion.text);
                });

                vscode.window.showInformationMessage(
                    `✅ Code completed using ${completion.provider} (${completion.model})`
                );

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Completion failed: ${error}`);
            }
        });
    });

    // Code analysis command
    const analyzeCommand = vscode.commands.registerCommand('uaida.analyze', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const text = editor.document.getText();
        if (!text.trim()) {
            vscode.window.showErrorMessage('No code to analyze');
            return;
        }

        const analysisType = await vscode.window.showQuickPick([
            { label: '🔒 Security', value: 'security' },
            { label: '⚡ Performance', value: 'performance' },
            { label: '✨ Quality', value: 'quality' },
            { label: '🐛 Bugs', value: 'bugs' },
            { label: '💡 Suggestions', value: 'suggestions' }
        ], { placeHolder: 'Select analysis type' });

        if (!analysisType) return;

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: `🔍 Analyzing code for ${analysisType.label}...`,
            cancellable: true
        }, async (progress, token) => {
            try {
                const analysis = await client.analyze({
                    code: text,
                    language: editor.document.languageId,
                    analysis_type: analysisType.value
                });

                if (token.isCancellationRequested) return;

                // Show results in new document
                const doc = await vscode.workspace.openTextDocument({
                    content: `# Code Analysis Results\n\n## ${analysisType.label}\n\n${analysis.summary}\n\n**Confidence:** ${(analysis.confidence_score * 100).toFixed(1)}%`,
                    language: 'markdown'
                });
                await vscode.window.showTextDocument(doc);

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Analysis failed: ${error}`);
            }
        });
    });

    // Code explanation command
    const explainCommand = vscode.commands.registerCommand('uaida.explain', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        if (selection.isEmpty) {
            vscode.window.showErrorMessage('Please select code to explain');
            return;
        }

        const text = editor.document.getText(selection);

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "🧠 Explaining code...",
            cancellable: true
        }, async (progress, token) => {
            try {
                const explanation = await client.codeAction({
                    code: text,
                    language: editor.document.languageId,
                    action: 'explain'
                });

                if (token.isCancellationRequested) return;

                // Show explanation in new document
                const doc = await vscode.workspace.openTextDocument({
                    content: `# Code Explanation\n\n## Selected Code\n\n\`\`\`${editor.document.languageId}\n${text}\n\`\`\`\n\n## Explanation\n\n${explanation.result}`,
                    language: 'markdown'
                });
                await vscode.window.showTextDocument(doc);

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Explanation failed: ${error}`);
            }
        });
    });

    // Documentation generation command
    const documentCommand = vscode.commands.registerCommand('uaida.document', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        const text = selection.isEmpty ? 
            editor.document.getText() : 
            editor.document.getText(selection);

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "📚 Generating documentation...",
            cancellable: true
        }, async (progress, token) => {
            try {
                const documentation = await client.codeAction({
                    code: text,
                    language: editor.document.languageId,
                    action: 'document'
                });

                if (token.isCancellationRequested) return;

                // Insert documentation above the code
                const position = selection.isEmpty ? 
                    new vscode.Position(0, 0) : 
                    selection.start;

                await editor.edit(editBuilder => {
                    editBuilder.insert(position, documentation.result + '\n\n');
                });

                vscode.window.showInformationMessage('✅ Documentation generated');

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Documentation generation failed: ${error}`);
            }
        });
    });

    // Test generation command
    const testCommand = vscode.commands.registerCommand('uaida.test', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        const text = selection.isEmpty ? 
            editor.document.getText() : 
            editor.document.getText(selection);

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "🧪 Generating tests...",
            cancellable: true
        }, async (progress, token) => {
            try {
                const tests = await client.codeAction({
                    code: text,
                    language: editor.document.languageId,
                    action: 'test'
                });

                if (token.isCancellationRequested) return;

                // Create new test file
                const fileName = editor.document.fileName;
                const testFileName = fileName.replace(/\.(js|ts|py|rs|go|java)$/, '.test.$1');
                
                const doc = await vscode.workspace.openTextDocument({
                    content: tests.result,
                    language: editor.document.languageId
                });
                await vscode.window.showTextDocument(doc);

                vscode.window.showInformationMessage('✅ Tests generated');

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Test generation failed: ${error}`);
            }
        });
    });

    // Refactoring command
    const refactorCommand = vscode.commands.registerCommand('uaida.refactor', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        if (selection.isEmpty) {
            vscode.window.showErrorMessage('Please select code to refactor');
            return;
        }

        const instructions = await vscode.window.showInputBox({
            prompt: 'Enter refactoring instructions',
            placeHolder: 'e.g., "Extract this into a separate function", "Optimize for performance"'
        });

        if (!instructions) return;

        const text = editor.document.getText(selection);

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "🔧 Refactoring code...",
            cancellable: true
        }, async (progress, token) => {
            try {
                const refactored = await client.codeAction({
                    code: text,
                    language: editor.document.languageId,
                    action: 'refactor',
                    instructions
                });

                if (token.isCancellationRequested) return;

                // Replace selected code
                await editor.edit(editBuilder => {
                    editBuilder.replace(selection, refactored.result);
                });

                vscode.window.showInformationMessage('✅ Code refactored');

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Refactoring failed: ${error}`);
            }
        });
    });

    // Code translation command
    const translateCommand = vscode.commands.registerCommand('uaida.translate', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showErrorMessage('No active editor');
            return;
        }

        const selection = editor.selection;
        if (selection.isEmpty) {
            vscode.window.showErrorMessage('Please select code to translate');
            return;
        }

        const targetLanguage = await vscode.window.showQuickPick([
            { label: '🦀 Rust', value: 'rust' },
            { label: '🐍 Python', value: 'python' },
            { label: '📜 JavaScript', value: 'javascript' },
            { label: '📘 TypeScript', value: 'typescript' },
            { label: '🐹 Go', value: 'go' },
            { label: '☕ Java', value: 'java' },
            { label: '⚡ C++', value: 'cpp' },
            { label: '🔧 C', value: 'c' }
        ], { placeHolder: 'Select target language' });

        if (!targetLanguage) return;

        const text = editor.document.getText(selection);

        await vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: `🔄 Translating to ${targetLanguage.label}...`,
            cancellable: true
        }, async (progress, token) => {
            try {
                const translated = await client.codeAction({
                    code: text,
                    language: editor.document.languageId,
                    action: 'translate',
                    target_language: targetLanguage.value
                });

                if (token.isCancellationRequested) return;

                // Show translated code in new document
                const doc = await vscode.workspace.openTextDocument({
                    content: translated.result,
                    language: targetLanguage.value
                });
                await vscode.window.showTextDocument(doc);

                vscode.window.showInformationMessage(`✅ Code translated to ${targetLanguage.label}`);

            } catch (error) {
                vscode.window.showErrorMessage(`❌ Translation failed: ${error}`);
            }
        });
    });

    // Chat command
    const chatCommand = vscode.commands.registerCommand('uaida.chat', () => {
        chatPanel.show();
    });

    // Status command
    const statusCommand = vscode.commands.registerCommand('uaida.status', async () => {
        try {
            const health = await client.health();
            const message = `UAIDA Status: ${health.status} (v${health.version})`;
            vscode.window.showInformationMessage(message);
        } catch (error) {
            vscode.window.showErrorMessage(`❌ Failed to get status: ${error}`);
        }
    });

    // Configuration command
    const configureCommand = vscode.commands.registerCommand('uaida.configure', () => {
        vscode.commands.executeCommand('workbench.action.openSettings', 'uaida');
    });

    // Register all commands
    context.subscriptions.push(
        completeCommand,
        analyzeCommand,
        explainCommand,
        documentCommand,
        testCommand,
        refactorCommand,
        translateCommand,
        chatCommand,
        statusCommand,
        configureCommand
    );
}

function registerProviders(context: vscode.ExtensionContext) {
    // Register completion provider
    const completionDisposable = vscode.languages.registerInlineCompletionItemProvider(
        { scheme: 'file' },
        completionProvider
    );

    // Register diagnostics
    context.subscriptions.push(completionDisposable);
}

function startBackgroundServices() {
    // Update status bar
    statusBar.update();
    
    // Start diagnostics if enabled
    const config = vscode.workspace.getConfiguration('uaida');
    if (config.get('enableDiagnostics', true)) {
        diagnosticsProvider.start();
    }
}

export function deactivate() {
    console.log('👋 UAIDA extension deactivated');
}
import * as vscode from 'vscode';
import { UAIDAClient } from '../client/uaidaClient';

export class AICompletionProvider implements vscode.CompletionItemProvider {
    private client: UAIDAClient;
    private lastRequestTime = 0;
    private debounceDelay = 300; // ms

    constructor(client: UAIDAClient) {
        this.client = client;
    }

    async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): Promise<vscode.CompletionItem[]> {
        // Check if auto-completion is enabled
        const config = vscode.workspace.getConfiguration('uaida');
        if (!config.get('enableAutoComplete', true)) {
            return [];
        }

        // Debounce requests
        const now = Date.now();
        if (now - this.lastRequestTime < this.debounceDelay) {
            return [];
        }
        this.lastRequestTime = now;

        // Check if cancellation was requested
        if (token.isCancellationRequested) {
            return [];
        }

        try {
            const code = document.getText();
            const offset = document.offsetAt(position);
            const maxSuggestions = config.get('maxSuggestions', 5);

            const suggestions = await this.client.getCompletion({
                code,
                language: document.languageId,
                cursor_position: offset
            });

            const completionItems: vscode.CompletionItem[] = suggestions
                .slice(0, maxSuggestions)
                .map((suggestion, index) => {
                    const item = new vscode.CompletionItem(
                        suggestion,
                        vscode.CompletionItemKind.Text
                    );
                    
                    item.insertText = suggestion;
                    item.detail = 'AI Suggestion';
                    item.documentation = new vscode.MarkdownString(
                        `AI-generated completion (${index + 1}/${suggestions.length})`
                    );
                    
                    // Set sort order to prioritize AI suggestions
                    item.sortText = `0${index.toString().padStart(3, '0')}`;
                    
                    return item;
                });

            return completionItems;
        } catch (error) {
            console.error('Completion provider error:', error);
            return [];
        }
    }

    resolveCompletionItem(
        item: vscode.CompletionItem,
        token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.CompletionItem> {
        // Add additional details if needed
        return item;
    }
}
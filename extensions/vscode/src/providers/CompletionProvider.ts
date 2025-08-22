import * as vscode from 'vscode';
import { UAIDAClient } from '../services/UAIDAClient';

export class CompletionProvider implements vscode.InlineCompletionItemProvider {
    constructor(private client: UAIDAClient) {}

    async provideInlineCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        context: vscode.InlineCompletionContext,
        token: vscode.CancellationToken
    ): Promise<vscode.InlineCompletionItem[] | vscode.InlineCompletionList | null> {
        
        // Get text before cursor
        const textBeforeCursor = document.getText(
            new vscode.Range(new vscode.Position(0, 0), position)
        );

        // Skip if text is too short
        if (textBeforeCursor.trim().length < 10) {
            return null;
        }

        try {
            const response = await this.client.getCompletion({
                prompt: textBeforeCursor,
                language: document.languageId,
                max_tokens: 100,
                temperature: 0.3,
            });

            if (response.suggestions.length === 0) {
                return null;
            }

            // Convert suggestions to VS Code completion items
            const completionItems = response.suggestions.map(suggestion => {
                const item = new vscode.InlineCompletionItem(
                    suggestion.text,
                    new vscode.Range(position, position)
                );
                
                // Add metadata as command
                item.command = {
                    command: 'uaida.logCompletion',
                    title: 'Log Completion',
                    arguments: [suggestion.metadata]
                };

                return item;
            });

            return completionItems;

        } catch (error) {
            console.error('UAIDA completion failed:', error);
            return null;
        }
    }
}
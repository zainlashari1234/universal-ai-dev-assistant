import * as vscode from 'vscode';
import { UAIDAClient, CompletionRequest } from './client';

export class CompletionProvider implements vscode.CompletionItemProvider, vscode.InlineCompletionItemProvider {
    constructor(private client: UAIDAClient) {}

    // Standard completion provider (Ctrl+Space)
    async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): Promise<vscode.CompletionItem[]> {
        const line = document.lineAt(position);
        const prefix = line.text.substring(0, position.character);
        
        // Only trigger for meaningful prefixes
        if (prefix.trim().length < 2) {
            return [];
        }

        try {
            const completion = await this.getCompletion(document, position, prefix);
            
            if (token.isCancellationRequested) {
                return [];
            }

            const item = new vscode.CompletionItem(
                completion.completion,
                vscode.CompletionItemKind.Snippet
            );

            item.detail = `UAIDA (${completion.provider_used})`;
            item.documentation = new vscode.MarkdownString(
                `**Cost:** $${completion.cost.toFixed(4)} | **Confidence:** ${(completion.confidence * 100).toFixed(1)}%\n\n` +
                `**Response Time:** ${completion.response_time_ms}ms | **Tokens:** ${completion.tokens_used}`
            );
            
            item.insertText = completion.completion;
            item.sortText = '0'; // High priority
            
            // Add additional suggestions as separate items
            const items = [item];
            completion.suggestions.forEach((suggestion, index) => {
                const suggestionItem = new vscode.CompletionItem(
                    suggestion,
                    vscode.CompletionItemKind.Text
                );
                suggestionItem.detail = `UAIDA Alternative ${index + 1}`;
                suggestionItem.sortText = `1${index}`;
                items.push(suggestionItem);
            });

            return items;
        } catch (error) {
            console.error('UAIDA completion error:', error);
            return [];
        }
    }

    // Inline completion provider (like GitHub Copilot)
    async provideInlineCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        context: vscode.InlineCompletionContext,
        token: vscode.CancellationToken
    ): Promise<vscode.InlineCompletionItem[]> {
        // Only provide inline completions if auto-complete is enabled
        if (!vscode.workspace.getConfiguration('uaida').get('autoComplete')) {
            return [];
        }

        const line = document.lineAt(position);
        const prefix = line.text.substring(0, position.character);
        
        // Trigger conditions for inline completion
        const shouldTrigger = 
            prefix.endsWith('//') ||  // Comment
            prefix.endsWith('/*') ||  // Block comment
            prefix.endsWith('function ') ||  // Function declaration
            prefix.endsWith('const ') ||  // Variable declaration
            prefix.endsWith('let ') ||
            prefix.endsWith('var ') ||
            prefix.endsWith('def ') ||  // Python function
            prefix.endsWith('fn ') ||   // Rust function
            prefix.endsWith('class ') ||  // Class declaration
            prefix.match(/\{\s*$/) ||  // Opening brace
            prefix.match(/\(\s*$/) ||  // Opening parenthesis
            context.triggerKind === vscode.InlineCompletionTriggerKind.Automatic;

        if (!shouldTrigger) {
            return [];
        }

        try {
            const completion = await this.getCompletion(document, position, prefix);
            
            if (token.isCancellationRequested) {
                return [];
            }

            const item = new vscode.InlineCompletionItem(
                completion.completion,
                new vscode.Range(position, position)
            );

            // Add command to show cost info
            item.command = {
                command: 'uaida.showCompletionInfo',
                title: 'Show UAIDA Info',
                arguments: [completion]
            };

            return [item];
        } catch (error) {
            console.error('UAIDA inline completion error:', error);
            return [];
        }
    }

    private async getCompletion(document: vscode.TextDocument, position: vscode.Position, prefix: string) {
        const request: CompletionRequest = {
            prompt: this.buildPrompt(document, position, prefix),
            language: document.languageId,
            context: this.getDocumentContext(document, position),
            max_tokens: vscode.workspace.getConfiguration('uaida').get('maxTokens'),
            temperature: vscode.workspace.getConfiguration('uaida').get('temperature')
        };

        return await this.client.completeCode(request);
    }

    private buildPrompt(document: vscode.TextDocument, position: vscode.Position, prefix: string): string {
        // Get context from previous lines
        const contextLines = [];
        const startLine = Math.max(0, position.line - 10);
        
        for (let i = startLine; i < position.line; i++) {
            contextLines.push(document.lineAt(i).text);
        }
        
        // Add current line up to cursor
        contextLines.push(prefix);
        
        // Create a meaningful prompt based on context
        const context = contextLines.join('\n');
        
        // Detect what kind of completion is needed
        if (prefix.includes('//') || prefix.includes('#')) {
            return `${context}\n// Continue this comment or add implementation`;
        } else if (prefix.includes('function') || prefix.includes('def') || prefix.includes('fn')) {
            return `${context}\n// Complete this function implementation`;
        } else if (prefix.includes('class')) {
            return `${context}\n// Complete this class definition`;
        } else {
            return `${context}\n// Complete this code`;
        }
    }

    private getDocumentContext(document: vscode.TextDocument, position: vscode.Position): string {
        // Get broader context for better completions
        const startLine = Math.max(0, position.line - 20);
        const endLine = Math.min(document.lineCount - 1, position.line + 5);
        
        const contextLines = [];
        for (let i = startLine; i <= endLine; i++) {
            if (i === position.line) {
                // Only include text before cursor on current line
                const line = document.lineAt(i);
                contextLines.push(line.text.substring(0, position.character));
            } else {
                contextLines.push(document.lineAt(i).text);
            }
        }
        
        return contextLines.join('\n');
    }
}
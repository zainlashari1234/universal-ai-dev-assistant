import * as vscode from 'vscode';
import { UAIDAClient } from '../services/UAIDAClient';

export class ChatItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly contextValue?: string
    ) {
        super(label, collapsibleState);
        this.tooltip = this.label;
    }
}

export class ChatProvider implements vscode.TreeDataProvider<ChatItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<ChatItem | undefined | null | void> = new vscode.EventEmitter<ChatItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<ChatItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private chatHistory: Array<{ role: string; content: string; timestamp: Date }> = [];

    constructor(private client: UAIDAClient) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: ChatItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: ChatItem): Thenable<ChatItem[]> {
        if (!element) {
            // Root level - show chat sessions
            return Promise.resolve([
                new ChatItem('New Chat', vscode.TreeItemCollapsibleState.None, 'newChat'),
                new ChatItem('Chat History', vscode.TreeItemCollapsibleState.Expanded, 'history')
            ]);
        } else if (element.contextValue === 'history') {
            // Show chat history
            return Promise.resolve(
                this.chatHistory.map((msg, index) => 
                    new ChatItem(
                        `${msg.role}: ${msg.content.substring(0, 50)}...`,
                        vscode.TreeItemCollapsibleState.None,
                        'chatMessage'
                    )
                )
            );
        }
        return Promise.resolve([]);
    }

    async sendChatMessage(message: string): Promise<void> {
        try {
            // Add user message to history
            this.chatHistory.push({
                role: 'user',
                content: message,
                timestamp: new Date()
            });

            // Get AI response
            const response = await this.client.sendChatMessage(message);
            
            // Add AI response to history
            this.chatHistory.push({
                role: 'assistant',
                content: response.content,
                timestamp: response.timestamp
            });

            this.refresh();
        } catch (error) {
            vscode.window.showErrorMessage(`Chat failed: ${error}`);
        }
    }
}
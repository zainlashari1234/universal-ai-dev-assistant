import * as vscode from 'vscode';
import { UAIDAClient } from './client';

export class ChatProvider implements vscode.TreeDataProvider<ChatItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<ChatItem | undefined | null | void> = new vscode.EventEmitter<ChatItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<ChatItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private chatHistory: ChatMessage[] = [];
    private webviewPanel: vscode.WebviewPanel | undefined;

    constructor(private client: UAIDAClient, private context: vscode.ExtensionContext) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: ChatItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: ChatItem): Thenable<ChatItem[]> {
        if (!element) {
            // Root level - show recent conversations
            return Promise.resolve([
                new ChatItem('💬 New Chat', 'Start a new conversation', vscode.TreeItemCollapsibleState.None, 'newChat'),
                new ChatItem('📊 Provider Status', 'Check AI provider status', vscode.TreeItemCollapsibleState.None, 'providerStatus'),
                new ChatItem('💰 Cost Summary', 'View usage costs', vscode.TreeItemCollapsibleState.None, 'costSummary'),
                new ChatItem('⚙️ Settings', 'Open UAIDA settings', vscode.TreeItemCollapsibleState.None, 'settings')
            ]);
        }
        return Promise.resolve([]);
    }

    showChatPanel() {
        if (this.webviewPanel) {
            this.webviewPanel.reveal();
            return;
        }

        this.webviewPanel = vscode.window.createWebviewPanel(
            'uaidaChat',
            'UAIDA AI Assistant',
            vscode.ViewColumn.Beside,
            {
                enableScripts: true,
                retainContextWhenHidden: true,
                localResourceRoots: [this.context.extensionUri]
            }
        );

        this.webviewPanel.webview.html = this.getChatWebviewContent();

        // Handle messages from webview
        this.webviewPanel.webview.onDidReceiveMessage(
            async (message) => {
                switch (message.type) {
                    case 'sendMessage':
                        await this.handleChatMessage(message.text);
                        break;
                    case 'clearChat':
                        this.chatHistory = [];
                        this.updateChatWebview();
                        break;
                    case 'insertCode':
                        await this.insertCodeIntoEditor(message.code);
                        break;
                }
            },
            undefined,
            this.context.subscriptions
        );

        this.webviewPanel.onDidDispose(() => {
            this.webviewPanel = undefined;
        });
    }

    private async handleChatMessage(message: string) {
        // Add user message to history
        this.chatHistory.push({
            role: 'user',
            content: message,
            timestamp: new Date()
        });

        this.updateChatWebview();

        try {
            // Get current editor context if available
            const editor = vscode.window.activeTextEditor;
            let context = '';
            
            if (editor) {
                const selection = editor.selection;
                if (!selection.isEmpty) {
                    context = editor.document.getText(selection);
                } else {
                    // Get some context around cursor
                    const position = selection.active;
                    const startLine = Math.max(0, position.line - 5);
                    const endLine = Math.min(editor.document.lineCount - 1, position.line + 5);
                    context = editor.document.getText(new vscode.Range(startLine, 0, endLine, 0));
                }
            }

            // Send to AI
            const response = await this.client.sendChatMessage(message, context);

            // Add AI response to history
            this.chatHistory.push({
                role: 'assistant',
                content: response,
                timestamp: new Date()
            });

            this.updateChatWebview();
        } catch (error) {
            vscode.window.showErrorMessage(`Chat error: ${error}`);
            
            // Add error message to chat
            this.chatHistory.push({
                role: 'assistant',
                content: `Sorry, I encountered an error: ${error}`,
                timestamp: new Date()
            });
            
            this.updateChatWebview();
        }
    }

    private updateChatWebview() {
        if (this.webviewPanel) {
            this.webviewPanel.webview.postMessage({
                type: 'updateChat',
                messages: this.chatHistory
            });
        }
    }

    private async insertCodeIntoEditor(code: string) {
        const editor = vscode.window.activeTextEditor;
        if (editor) {
            await editor.edit(editBuilder => {
                editBuilder.insert(editor.selection.active, code);
            });
            vscode.window.showInformationMessage('Code inserted!');
        } else {
            vscode.window.showWarningMessage('No active editor to insert code');
        }
    }

    private getChatWebviewContent(): string {
        return `
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>UAIDA Chat</title>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    background-color: var(--vscode-editor-background);
                    color: var(--vscode-editor-foreground);
                    margin: 0;
                    padding: 10px;
                    height: 100vh;
                    display: flex;
                    flex-direction: column;
                }
                
                .chat-container {
                    flex: 1;
                    overflow-y: auto;
                    padding: 10px;
                    border: 1px solid var(--vscode-panel-border);
                    border-radius: 4px;
                    margin-bottom: 10px;
                }
                
                .message {
                    margin-bottom: 15px;
                    padding: 10px;
                    border-radius: 8px;
                    max-width: 80%;
                }
                
                .user-message {
                    background-color: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    margin-left: auto;
                    text-align: right;
                }
                
                .assistant-message {
                    background-color: var(--vscode-input-background);
                    border: 1px solid var(--vscode-input-border);
                }
                
                .message-header {
                    font-size: 0.8em;
                    opacity: 0.7;
                    margin-bottom: 5px;
                }
                
                .message-content {
                    white-space: pre-wrap;
                    word-wrap: break-word;
                }
                
                .code-block {
                    background-color: var(--vscode-textCodeBlock-background);
                    border: 1px solid var(--vscode-panel-border);
                    border-radius: 4px;
                    padding: 10px;
                    margin: 10px 0;
                    font-family: var(--vscode-editor-font-family);
                    position: relative;
                }
                
                .code-actions {
                    position: absolute;
                    top: 5px;
                    right: 5px;
                }
                
                .code-action-btn {
                    background: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    border: none;
                    padding: 4px 8px;
                    border-radius: 3px;
                    cursor: pointer;
                    font-size: 0.8em;
                    margin-left: 5px;
                }
                
                .input-container {
                    display: flex;
                    gap: 10px;
                }
                
                .message-input {
                    flex: 1;
                    padding: 10px;
                    background-color: var(--vscode-input-background);
                    color: var(--vscode-input-foreground);
                    border: 1px solid var(--vscode-input-border);
                    border-radius: 4px;
                    font-family: var(--vscode-font-family);
                    resize: vertical;
                    min-height: 40px;
                }
                
                .send-btn, .clear-btn {
                    padding: 10px 15px;
                    background-color: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                    font-family: var(--vscode-font-family);
                }
                
                .send-btn:hover, .clear-btn:hover, .code-action-btn:hover {
                    background-color: var(--vscode-button-hoverBackground);
                }
                
                .typing-indicator {
                    font-style: italic;
                    opacity: 0.7;
                    padding: 10px;
                }
            </style>
        </head>
        <body>
            <div class="chat-container" id="chatContainer">
                <div class="message assistant-message">
                    <div class="message-header">🤖 UAIDA Assistant</div>
                    <div class="message-content">Hello! I'm your AI development assistant. I can help you with:
                    
• Code completion and generation
• Code analysis and review
• Bug detection and fixes
• Performance optimization
• Security analysis
• Documentation generation

How can I help you today?</div>
                </div>
            </div>
            
            <div class="input-container">
                <textarea 
                    id="messageInput" 
                    class="message-input" 
                    placeholder="Ask me anything about your code..."
                    rows="2"
                ></textarea>
                <button id="sendBtn" class="send-btn">Send</button>
                <button id="clearBtn" class="clear-btn">Clear</button>
            </div>
            
            <script>
                const vscode = acquireVsCodeApi();
                const chatContainer = document.getElementById('chatContainer');
                const messageInput = document.getElementById('messageInput');
                const sendBtn = document.getElementById('sendBtn');
                const clearBtn = document.getElementById('clearBtn');
                
                let isTyping = false;
                
                sendBtn.addEventListener('click', sendMessage);
                clearBtn.addEventListener('click', clearChat);
                
                messageInput.addEventListener('keydown', (e) => {
                    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
                        sendMessage();
                    }
                });
                
                function sendMessage() {
                    const message = messageInput.value.trim();
                    if (!message || isTyping) return;
                    
                    vscode.postMessage({
                        type: 'sendMessage',
                        text: message
                    });
                    
                    messageInput.value = '';
                    showTypingIndicator();
                }
                
                function clearChat() {
                    vscode.postMessage({
                        type: 'clearChat'
                    });
                }
                
                function showTypingIndicator() {
                    isTyping = true;
                    const indicator = document.createElement('div');
                    indicator.className = 'typing-indicator';
                    indicator.textContent = '🤖 UAIDA is thinking...';
                    indicator.id = 'typingIndicator';
                    chatContainer.appendChild(indicator);
                    chatContainer.scrollTop = chatContainer.scrollHeight;
                }
                
                function hideTypingIndicator() {
                    isTyping = false;
                    const indicator = document.getElementById('typingIndicator');
                    if (indicator) {
                        indicator.remove();
                    }
                }
                
                function insertCode(code) {
                    vscode.postMessage({
                        type: 'insertCode',
                        code: code
                    });
                }
                
                function formatMessage(content) {
                    // Simple code block detection and formatting
                    return content.replace(/\`\`\`([\\s\\S]*?)\`\`\`/g, (match, code) => {
                        return \`<div class="code-block">
                            <div class="code-actions">
                                <button class="code-action-btn" onclick="insertCode(\\\`\${code.trim()}\\\`)">Insert</button>
                                <button class="code-action-btn" onclick="navigator.clipboard.writeText(\\\`\${code.trim()}\\\`)">Copy</button>
                            </div>
                            <pre><code>\${code.trim()}</code></pre>
                        </div>\`;
                    });
                }
                
                // Handle messages from extension
                window.addEventListener('message', event => {
                    const message = event.data;
                    
                    switch (message.type) {
                        case 'updateChat':
                            hideTypingIndicator();
                            updateChatDisplay(message.messages);
                            break;
                    }
                });
                
                function updateChatDisplay(messages) {
                    // Keep the welcome message and clear the rest
                    const welcomeMessage = chatContainer.children[0];
                    chatContainer.innerHTML = '';
                    chatContainer.appendChild(welcomeMessage);
                    
                    messages.forEach(msg => {
                        const messageDiv = document.createElement('div');
                        messageDiv.className = \`message \${msg.role}-message\`;
                        
                        const headerDiv = document.createElement('div');
                        headerDiv.className = 'message-header';
                        headerDiv.textContent = msg.role === 'user' ? '👤 You' : '🤖 UAIDA';
                        
                        const contentDiv = document.createElement('div');
                        contentDiv.className = 'message-content';
                        contentDiv.innerHTML = formatMessage(msg.content);
                        
                        messageDiv.appendChild(headerDiv);
                        messageDiv.appendChild(contentDiv);
                        chatContainer.appendChild(messageDiv);
                    });
                    
                    chatContainer.scrollTop = chatContainer.scrollHeight;
                }
            </script>
        </body>
        </html>`;
    }
}

class ChatItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly tooltip: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly command?: string
    ) {
        super(label, collapsibleState);
        this.tooltip = tooltip;
        
        if (command) {
            this.command = {
                command: `uaida.${command}`,
                title: label
            };
        }
    }
}

interface ChatMessage {
    role: 'user' | 'assistant';
    content: string;
    timestamp: Date;
}
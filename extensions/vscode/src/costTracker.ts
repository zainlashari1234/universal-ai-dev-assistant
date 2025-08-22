import * as vscode from 'vscode';
import { UAIDAClient } from './client';

export class CostTracker implements vscode.TreeDataProvider<CostItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<CostItem | undefined | null | void> = new vscode.EventEmitter<CostItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<CostItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private dailyCosts: Map<string, number> = new Map();
    private providerUsage: Map<string, ProviderUsage> = new Map();
    private totalCost: number = 0;
    private totalTokens: number = 0;
    private totalRequests: number = 0;

    constructor(private client: UAIDAClient) {
        this.loadStoredData();
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: CostItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: CostItem): Thenable<CostItem[]> {
        if (!element) {
            // Root level - show cost summary
            return Promise.resolve([
                new CostItem(
                    `💰 Total Cost: $${this.totalCost.toFixed(4)}`,
                    `Total spent across all providers`,
                    vscode.TreeItemCollapsibleState.None
                ),
                new CostItem(
                    `🔢 Total Tokens: ${this.totalTokens.toLocaleString()}`,
                    `Total tokens used`,
                    vscode.TreeItemCollapsibleState.None
                ),
                new CostItem(
                    `📊 Total Requests: ${this.totalRequests}`,
                    `Total API requests made`,
                    vscode.TreeItemCollapsibleState.None
                ),
                new CostItem(
                    `📅 Today: $${this.getTodayCost().toFixed(4)}`,
                    `Cost for today`,
                    vscode.TreeItemCollapsibleState.None
                ),
                new CostItem(
                    `🔌 Providers`,
                    `Usage by provider`,
                    vscode.TreeItemCollapsibleState.Expanded,
                    'providers'
                ),
                new CostItem(
                    `📈 Daily Breakdown`,
                    `Daily cost breakdown`,
                    vscode.TreeItemCollapsibleState.Collapsed,
                    'daily'
                )
            ]);
        } else if (element.contextValue === 'providers') {
            // Show provider breakdown
            const items: CostItem[] = [];
            for (const [provider, usage] of this.providerUsage) {
                items.push(new CostItem(
                    `${this.getProviderIcon(provider)} ${provider}`,
                    `$${usage.cost.toFixed(4)} | ${usage.requests} requests | ${usage.tokens.toLocaleString()} tokens`,
                    vscode.TreeItemCollapsibleState.None
                ));
            }
            return Promise.resolve(items);
        } else if (element.contextValue === 'daily') {
            // Show daily breakdown
            const items: CostItem[] = [];
            const sortedDays = Array.from(this.dailyCosts.entries())
                .sort(([a], [b]) => b.localeCompare(a))
                .slice(0, 7); // Last 7 days
            
            for (const [date, cost] of sortedDays) {
                items.push(new CostItem(
                    `📅 ${date}`,
                    `$${cost.toFixed(4)}`,
                    vscode.TreeItemCollapsibleState.None
                ));
            }
            return Promise.resolve(items);
        }
        
        return Promise.resolve([]);
    }

    addUsage(cost: number, tokens: number, provider: string) {
        // Update totals
        this.totalCost += cost;
        this.totalTokens += tokens;
        this.totalRequests += 1;

        // Update provider usage
        const existing = this.providerUsage.get(provider) || { cost: 0, tokens: 0, requests: 0 };
        existing.cost += cost;
        existing.tokens += tokens;
        existing.requests += 1;
        this.providerUsage.set(provider, existing);

        // Update daily costs
        const today = new Date().toISOString().split('T')[0];
        const dailyCost = this.dailyCosts.get(today) || 0;
        this.dailyCosts.set(today, dailyCost + cost);

        // Save to storage
        this.saveData();
        
        // Refresh UI
        this.refresh();

        // Show warning if cost is getting high
        if (this.getTodayCost() > 5.0) {
            vscode.window.showWarningMessage(
                `💰 High usage today: $${this.getTodayCost().toFixed(2)}. Consider using Ollama (free) for basic tasks.`,
                'Switch to Ollama', 'View Costs'
            ).then(selection => {
                if (selection === 'Switch to Ollama') {
                    vscode.workspace.getConfiguration('uaida').update('defaultProvider', 'ollama', true);
                } else if (selection === 'View Costs') {
                    this.showDashboard();
                }
            });
        }
    }

    showDashboard() {
        const panel = vscode.window.createWebviewPanel(
            'uaidaCosts',
            'UAIDA Cost Dashboard',
            vscode.ViewColumn.One,
            { enableScripts: true }
        );

        panel.webview.html = this.getDashboardHtml();
    }

    private getTodayCost(): number {
        const today = new Date().toISOString().split('T')[0];
        return this.dailyCosts.get(today) || 0;
    }

    private getProviderIcon(provider: string): string {
        const icons: { [key: string]: string } = {
            'openai': '🤖',
            'openrouter': '🔀',
            'ollama': '🏠',
            'anthropic': '🧠',
            'cohere': '🌊',
            'together': '🤝'
        };
        return icons[provider] || '🔌';
    }

    private saveData() {
        const data = {
            totalCost: this.totalCost,
            totalTokens: this.totalTokens,
            totalRequests: this.totalRequests,
            dailyCosts: Array.from(this.dailyCosts.entries()),
            providerUsage: Array.from(this.providerUsage.entries())
        };
        
        // In a real extension, you'd use context.globalState or context.workspaceState
        // For now, we'll use a simple in-memory storage
        (global as any).uaidaCostData = data;
    }

    private loadStoredData() {
        const data = (global as any).uaidaCostData;
        if (data) {
            this.totalCost = data.totalCost || 0;
            this.totalTokens = data.totalTokens || 0;
            this.totalRequests = data.totalRequests || 0;
            this.dailyCosts = new Map(data.dailyCosts || []);
            this.providerUsage = new Map(data.providerUsage || []);
        }
    }

    private getDashboardHtml(): string {
        const providerData = Array.from(this.providerUsage.entries()).map(([name, usage]) => ({
            name,
            cost: usage.cost,
            requests: usage.requests,
            tokens: usage.tokens
        }));

        const dailyData = Array.from(this.dailyCosts.entries())
            .sort(([a], [b]) => a.localeCompare(b))
            .slice(-30); // Last 30 days

        return `
        <!DOCTYPE html>
        <html>
        <head>
            <title>UAIDA Cost Dashboard</title>
            <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    background-color: var(--vscode-editor-background);
                    color: var(--vscode-editor-foreground);
                    padding: 20px;
                }
                .dashboard-grid {
                    display: grid;
                    grid-template-columns: 1fr 1fr;
                    gap: 20px;
                    margin-bottom: 30px;
                }
                .metric-card {
                    background: var(--vscode-input-background);
                    border: 1px solid var(--vscode-input-border);
                    border-radius: 8px;
                    padding: 20px;
                    text-align: center;
                }
                .metric-value {
                    font-size: 2em;
                    font-weight: bold;
                    color: var(--vscode-button-background);
                }
                .metric-label {
                    font-size: 0.9em;
                    opacity: 0.8;
                    margin-top: 5px;
                }
                .chart-container {
                    background: var(--vscode-input-background);
                    border: 1px solid var(--vscode-input-border);
                    border-radius: 8px;
                    padding: 20px;
                    margin-bottom: 20px;
                }
                .provider-table {
                    width: 100%;
                    border-collapse: collapse;
                    margin-top: 20px;
                }
                .provider-table th,
                .provider-table td {
                    padding: 10px;
                    text-align: left;
                    border-bottom: 1px solid var(--vscode-input-border);
                }
                .provider-table th {
                    background: var(--vscode-button-background);
                    color: var(--vscode-button-foreground);
                }
                .cost-warning {
                    background: var(--vscode-inputValidation-warningBackground);
                    border: 1px solid var(--vscode-inputValidation-warningBorder);
                    color: var(--vscode-inputValidation-warningForeground);
                    padding: 15px;
                    border-radius: 4px;
                    margin-bottom: 20px;
                }
            </style>
        </head>
        <body>
            <h1>💰 UAIDA Cost Dashboard</h1>
            
            ${this.getTodayCost() > 5.0 ? `
            <div class="cost-warning">
                ⚠️ High usage detected today ($${this.getTodayCost().toFixed(2)}). 
                Consider using Ollama (free) for basic tasks to reduce costs.
            </div>
            ` : ''}
            
            <div class="dashboard-grid">
                <div class="metric-card">
                    <div class="metric-value">$${this.totalCost.toFixed(4)}</div>
                    <div class="metric-label">Total Cost</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${this.totalTokens.toLocaleString()}</div>
                    <div class="metric-label">Total Tokens</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">${this.totalRequests}</div>
                    <div class="metric-label">Total Requests</div>
                </div>
                <div class="metric-card">
                    <div class="metric-value">$${this.getTodayCost().toFixed(4)}</div>
                    <div class="metric-label">Today's Cost</div>
                </div>
            </div>
            
            <div class="chart-container">
                <h3>📈 Daily Cost Trend</h3>
                <canvas id="dailyChart" width="400" height="200"></canvas>
            </div>
            
            <div class="chart-container">
                <h3>🔌 Provider Usage</h3>
                <canvas id="providerChart" width="400" height="200"></canvas>
            </div>
            
            <div class="chart-container">
                <h3>📊 Provider Details</h3>
                <table class="provider-table">
                    <thead>
                        <tr>
                            <th>Provider</th>
                            <th>Cost</th>
                            <th>Requests</th>
                            <th>Tokens</th>
                            <th>Avg Cost/Request</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${providerData.map(p => `
                        <tr>
                            <td>${this.getProviderIcon(p.name)} ${p.name}</td>
                            <td>$${p.cost.toFixed(4)}</td>
                            <td>${p.requests}</td>
                            <td>${p.tokens.toLocaleString()}</td>
                            <td>$${(p.cost / p.requests).toFixed(4)}</td>
                        </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
            
            <script>
                // Daily cost chart
                const dailyCtx = document.getElementById('dailyChart').getContext('2d');
                new Chart(dailyCtx, {
                    type: 'line',
                    data: {
                        labels: ${JSON.stringify(dailyData.map(([date]) => date))},
                        datasets: [{
                            label: 'Daily Cost ($)',
                            data: ${JSON.stringify(dailyData.map(([, cost]) => cost))},
                            borderColor: 'rgb(75, 192, 192)',
                            backgroundColor: 'rgba(75, 192, 192, 0.2)',
                            tension: 0.1
                        }]
                    },
                    options: {
                        responsive: true,
                        scales: {
                            y: {
                                beginAtZero: true
                            }
                        }
                    }
                });
                
                // Provider usage chart
                const providerCtx = document.getElementById('providerChart').getContext('2d');
                new Chart(providerCtx, {
                    type: 'doughnut',
                    data: {
                        labels: ${JSON.stringify(providerData.map(p => p.name))},
                        datasets: [{
                            data: ${JSON.stringify(providerData.map(p => p.cost))},
                            backgroundColor: [
                                '#FF6384',
                                '#36A2EB',
                                '#FFCE56',
                                '#4BC0C0',
                                '#9966FF',
                                '#FF9F40'
                            ]
                        }]
                    },
                    options: {
                        responsive: true,
                        plugins: {
                            legend: {
                                position: 'bottom'
                            }
                        }
                    }
                });
            </script>
        </body>
        </html>`;
    }
}

class CostItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly tooltip: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly contextValue?: string
    ) {
        super(label, collapsibleState);
        this.tooltip = tooltip;
        this.contextValue = contextValue;
    }
}

interface ProviderUsage {
    cost: number;
    tokens: number;
    requests: number;
}
import * as vscode from 'vscode';
import { UAIDAClient } from './client';

export class UAIDAExplorer implements vscode.TreeDataProvider<UAIDAItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<UAIDAItem | undefined | null | void> = new vscode.EventEmitter<UAIDAItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<UAIDAItem | undefined | null | void> = this._onDidChangeTreeData.event;

    private activePlans: Map<string, any> = new Map();
    private activePatches: Map<string, any> = new Map();
    private testResults: Map<string, any> = new Map();

    constructor(private client: UAIDAClient) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: UAIDAItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: UAIDAItem): Thenable<UAIDAItem[]> {
        if (!element) {
            // Root level items
            return Promise.resolve([
                new UAIDAItem('Plans', vscode.TreeItemCollapsibleState.Expanded, 'plans'),
                new UAIDAItem('Patches', vscode.TreeItemCollapsibleState.Expanded, 'patches'),
                new UAIDAItem('Test Results', vscode.TreeItemCollapsibleState.Expanded, 'tests'),
                new UAIDAItem('Actions', vscode.TreeItemCollapsibleState.Expanded, 'actions'),
            ]);
        }

        switch (element.contextValue) {
            case 'plans':
                return this.getPlansChildren();
            case 'patches':
                return this.getPatchesChildren();
            case 'tests':
                return this.getTestsChildren();
            case 'actions':
                return this.getActionsChildren();
            default:
                return Promise.resolve([]);
        }
    }

    private async getPlansChildren(): Promise<UAIDAItem[]> {
        const items: UAIDAItem[] = [];
        
        for (const [id, plan] of this.activePlans) {
            const item = new UAIDAItem(
                `${plan.goal.substring(0, 30)}...`,
                vscode.TreeItemCollapsibleState.None,
                'plan'
            );
            item.description = `Risk: ${plan.risk_level}`;
            item.tooltip = `Plan ID: ${id}\nGoal: ${plan.goal}\nSteps: ${plan.steps.length}`;
            item.command = {
                command: 'uaida.showPlan',
                title: 'Show Plan',
                arguments: [id]
            };
            items.push(item);
        }

        if (items.length === 0) {
            const emptyItem = new UAIDAItem(
                'No active plans',
                vscode.TreeItemCollapsibleState.None,
                'empty'
            );
            emptyItem.description = 'Create a plan to get started';
            items.push(emptyItem);
        }

        return items;
    }

    private async getPatchesChildren(): Promise<UAIDAItem[]> {
        const items: UAIDAItem[] = [];
        
        for (const [id, patch] of this.activePatches) {
            const item = new UAIDAItem(
                `Patch ${id.substring(0, 8)}`,
                vscode.TreeItemCollapsibleState.None,
                'patch'
            );
            item.description = patch.success ? '✓ Applied' : '✗ Failed';
            item.tooltip = `Patch ID: ${id}\nFiles: ${patch.changes_applied?.length || 0}\nSuccess: ${patch.success}`;
            item.command = {
                command: 'uaida.showPatch',
                title: 'Show Patch',
                arguments: [id]
            };
            items.push(item);
        }

        if (items.length === 0) {
            const emptyItem = new UAIDAItem(
                'No patches applied',
                vscode.TreeItemCollapsibleState.None,
                'empty'
            );
            items.push(emptyItem);
        }

        return items;
    }

    private async getTestsChildren(): Promise<UAIDAItem[]> {
        const items: UAIDAItem[] = [];
        
        for (const [id, result] of this.testResults) {
            const item = new UAIDAItem(
                `Run ${id.substring(0, 8)}`,
                vscode.TreeItemCollapsibleState.None,
                'test'
            );
            item.description = result.success ? 
                `✓ ${result.tests_passed}/${result.tests_passed + result.tests_failed}` :
                `✗ ${result.tests_failed} failed`;
            item.tooltip = `Run ID: ${id}\nPassed: ${result.tests_passed}\nFailed: ${result.tests_failed}`;
            item.command = {
                command: 'uaida.showTestResults',
                title: 'Show Test Results',
                arguments: [id]
            };
            items.push(item);
        }

        if (items.length === 0) {
            const emptyItem = new UAIDAItem(
                'No test results',
                vscode.TreeItemCollapsibleState.None,
                'empty'
            );
            items.push(emptyItem);
        }

        return items;
    }

    private async getActionsChildren(): Promise<UAIDAItem[]> {
        return [
            new UAIDAItem('Create Plan', vscode.TreeItemCollapsibleState.None, 'action', {
                command: 'uaida.plan',
                title: 'Create Plan'
            }),
            new UAIDAItem('Apply Patch', vscode.TreeItemCollapsibleState.None, 'action', {
                command: 'uaida.patch',
                title: 'Apply Patch'
            }),
            new UAIDAItem('Run Tests', vscode.TreeItemCollapsibleState.None, 'action', {
                command: 'uaida.test',
                title: 'Run Tests'
            }),
            new UAIDAItem('Code Review', vscode.TreeItemCollapsibleState.None, 'action', {
                command: 'uaida.review',
                title: 'Code Review'
            }),
            new UAIDAItem('Rollback', vscode.TreeItemCollapsibleState.None, 'action', {
                command: 'uaida.rollback',
                title: 'Rollback'
            }),
        ];
    }

    addPlan(id: string, plan: any) {
        this.activePlans.set(id, plan);
        this.refresh();
    }

    addPatch(id: string, patch: any) {
        this.activePatches.set(id, patch);
        this.refresh();
    }

    addTestResult(id: string, result: any) {
        this.testResults.set(id, result);
        this.refresh();
    }

    clearAll() {
        this.activePlans.clear();
        this.activePatches.clear();
        this.testResults.clear();
        this.refresh();
    }
}

class UAIDAItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly contextValue: string,
        command?: vscode.Command
    ) {
        super(label, collapsibleState);
        this.command = command;
        
        // Set icons based on context
        switch (contextValue) {
            case 'plans':
                this.iconPath = new vscode.ThemeIcon('lightbulb');
                break;
            case 'patches':
                this.iconPath = new vscode.ThemeIcon('git-commit');
                break;
            case 'tests':
                this.iconPath = new vscode.ThemeIcon('beaker');
                break;
            case 'actions':
                this.iconPath = new vscode.ThemeIcon('tools');
                break;
            case 'plan':
                this.iconPath = new vscode.ThemeIcon('file-text');
                break;
            case 'patch':
                this.iconPath = new vscode.ThemeIcon('diff');
                break;
            case 'test':
                this.iconPath = new vscode.ThemeIcon('check');
                break;
            case 'action':
                this.iconPath = new vscode.ThemeIcon('play');
                break;
            case 'empty':
                this.iconPath = new vscode.ThemeIcon('info');
                break;
        }
    }
}
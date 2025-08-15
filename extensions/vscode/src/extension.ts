import * as vscode from 'vscode';
import { UAIDAClient } from './client';
import { DiffProvider } from './diffProvider';
import { UAIDAExplorer } from './explorer';
import { StatusBarManager } from './statusBar';

let client: UAIDAClient;
let diffProvider: DiffProvider;
let explorer: UAIDAExplorer;
let statusBar: StatusBarManager;

export function activate(context: vscode.ExtensionContext) {
    console.log('UAIDA Extension is now active!');

    // Initialize components
    client = new UAIDAClient();
    diffProvider = new DiffProvider();
    explorer = new UAIDAExplorer(client);
    statusBar = new StatusBarManager();

    // Register commands
    const commands = [
        vscode.commands.registerCommand('uaida.plan', handlePlanCommand),
        vscode.commands.registerCommand('uaida.patch', handlePatchCommand),
        vscode.commands.registerCommand('uaida.test', handleTestCommand),
        vscode.commands.registerCommand('uaida.review', handleReviewCommand),
        vscode.commands.registerCommand('uaida.rollback', handleRollbackCommand),
        vscode.commands.registerCommand('uaida.showDiff', handleShowDiffCommand),
    ];

    // Register providers
    const providers = [
        vscode.window.registerTreeDataProvider('uaidaExplorer', explorer),
        vscode.workspace.registerTextDocumentContentProvider('uaida-diff', diffProvider),
    ];

    // Add to context
    context.subscriptions.push(...commands, ...providers, statusBar);

    // Show welcome message
    vscode.window.showInformationMessage('UAIDA: AI Development Assistant is ready!');
}

async function handlePlanCommand() {
    try {
        statusBar.showProgress('Creating plan...');
        
        const goal = await vscode.window.showInputBox({
            prompt: 'What would you like to implement?',
            placeholder: 'e.g., Add error handling to division function',
            validateInput: (value) => {
                if (!value || value.trim().length === 0) {
                    return 'Please enter a goal';
                }
                return null;
            }
        });

        if (!goal) {
            statusBar.hide();
            return;
        }

        // Get current workspace context
        const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
        if (!workspaceFolder) {
            vscode.window.showErrorMessage('Please open a workspace folder');
            statusBar.hide();
            return;
        }

        // Get selected files or current file
        const selectedFiles = await getSelectedFiles();
        
        const planRequest = {
            goal: goal.trim(),
            context: {
                files: selectedFiles,
                constraints: {
                    max_files: 10,
                    max_loc: 1000,
                    timeout_s: 300
                }
            }
        };

        const planResponse = await client.createPlan(planRequest);
        
        if (planResponse.success) {
            await showPlanResult(planResponse.data);
            statusBar.showSuccess('Plan created successfully');
        } else {
            vscode.window.showErrorMessage(`Plan creation failed: ${planResponse.error}`);
            statusBar.showError('Plan creation failed');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Error: ${error}`);
        statusBar.showError('Plan creation failed');
    }
}

async function handlePatchCommand() {
    try {
        statusBar.showProgress('Applying patch...');

        // Get the current plan ID (simplified - in real implementation, would track active plans)
        const planId = await vscode.window.showInputBox({
            prompt: 'Enter Plan ID (or leave empty for latest)',
            placeholder: 'plan_12345...'
        });

        if (planId === undefined) {
            statusBar.hide();
            return;
        }

        // Get target files
        const targetFiles = await getSelectedFiles();
        
        const patchRequest = {
            plan_id: planId || 'latest',
            target_files: targetFiles,
            changes: [] // Would be populated based on plan
        };

        const patchResponse = await client.applyPatch(patchRequest);
        
        if (patchResponse.success) {
            const shouldShowDiff = vscode.workspace.getConfiguration('uaida').get('showDiffPreview', true);
            
            if (shouldShowDiff) {
                await showPatchDiff(patchResponse.data);
            }
            
            const action = await vscode.window.showInformationMessage(
                `Patch applied successfully. ${patchResponse.data.changes_applied.length} files changed.`,
                'View Changes',
                'Rollback'
            );
            
            if (action === 'View Changes') {
                await handleShowDiffCommand();
            } else if (action === 'Rollback') {
                await handleRollbackCommand();
            }
            
            statusBar.showSuccess('Patch applied');
        } else {
            vscode.window.showErrorMessage(`Patch failed: ${patchResponse.error}`);
            statusBar.showError('Patch failed');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Error: ${error}`);
        statusBar.showError('Patch failed');
    }
}

async function handleTestCommand() {
    try {
        statusBar.showProgress('Running tests...');

        const patchId = await vscode.window.showInputBox({
            prompt: 'Enter Patch ID (or leave empty for latest)',
            placeholder: 'patch_12345...'
        });

        if (patchId === undefined) {
            statusBar.hide();
            return;
        }

        const testFiles = await getTestFiles();
        
        const testRequest = {
            patch_id: patchId || 'latest',
            test_command: undefined, // Use default
            test_files: testFiles,
            timeout_s: 300
        };

        const testResponse = await client.runTests(testRequest);
        
        if (testResponse.success) {
            await showTestResults(testResponse.data);
            
            if (testResponse.data.success) {
                statusBar.showSuccess(`Tests passed: ${testResponse.data.tests_passed}/${testResponse.data.tests_passed + testResponse.data.tests_failed}`);
            } else {
                statusBar.showError(`Tests failed: ${testResponse.data.tests_failed} failures`);
            }
        } else {
            vscode.window.showErrorMessage(`Test execution failed: ${testResponse.error}`);
            statusBar.showError('Test execution failed');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Error: ${error}`);
        statusBar.showError('Test execution failed');
    }
}

async function handleReviewCommand() {
    try {
        statusBar.showProgress('Reviewing code...');

        const patchId = await vscode.window.showInputBox({
            prompt: 'Enter Patch ID for review',
            placeholder: 'patch_12345...'
        });

        if (!patchId) {
            statusBar.hide();
            return;
        }

        const reviewResponse = await client.getRiskReport(patchId);
        
        if (reviewResponse.success) {
            await showReviewResults(reviewResponse.data);
            statusBar.showSuccess('Review completed');
        } else {
            vscode.window.showErrorMessage(`Review failed: ${reviewResponse.error}`);
            statusBar.showError('Review failed');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Error: ${error}`);
        statusBar.showError('Review failed');
    }
}

async function handleRollbackCommand() {
    try {
        const patchId = await vscode.window.showInputBox({
            prompt: 'Enter Patch ID to rollback',
            placeholder: 'patch_12345...'
        });

        if (!patchId) {
            return;
        }

        const reason = await vscode.window.showInputBox({
            prompt: 'Reason for rollback',
            placeholder: 'e.g., Tests failed, Performance issues'
        });

        if (!reason) {
            return;
        }

        const confirmed = await vscode.window.showWarningMessage(
            `Are you sure you want to rollback patch ${patchId}?`,
            'Yes, Rollback',
            'Cancel'
        );

        if (confirmed !== 'Yes, Rollback') {
            return;
        }

        statusBar.showProgress('Rolling back...');

        const rollbackRequest = {
            patch_id: patchId,
            reason: reason
        };

        const rollbackResponse = await client.rollback(rollbackRequest);
        
        if (rollbackResponse.success) {
            vscode.window.showInformationMessage(
                `Rollback successful. ${rollbackResponse.data.restored_files.length} files restored.`
            );
            statusBar.showSuccess('Rollback completed');
        } else {
            vscode.window.showErrorMessage(`Rollback failed: ${rollbackResponse.error}`);
            statusBar.showError('Rollback failed');
        }
    } catch (error) {
        vscode.window.showErrorMessage(`Error: ${error}`);
        statusBar.showError('Rollback failed');
    }
}

async function handleShowDiffCommand() {
    try {
        const uri = vscode.Uri.parse('uaida-diff://latest');
        const doc = await vscode.workspace.openTextDocument(uri);
        await vscode.window.showTextDocument(doc);
    } catch (error) {
        vscode.window.showErrorMessage(`Error showing diff: ${error}`);
    }
}

async function getSelectedFiles(): Promise<string[]> {
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor) {
        return [vscode.workspace.asRelativePath(activeEditor.document.uri)];
    }
    
    // If no active editor, return common files
    return ['src/', 'tests/'];
}

async function getTestFiles(): Promise<string[]> {
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
    if (!workspaceFolder) {
        return [];
    }

    const testPattern = new vscode.RelativePattern(workspaceFolder, '**/test*.{py,js,ts}');
    const testFiles = await vscode.workspace.findFiles(testPattern);
    
    return testFiles.map(file => vscode.workspace.asRelativePath(file));
}

async function showPlanResult(plan: any) {
    const panel = vscode.window.createWebviewPanel(
        'uaidaPlan',
        'UAIDA Plan',
        vscode.ViewColumn.Two,
        { enableScripts: true }
    );

    panel.webview.html = generatePlanHTML(plan);
}

async function showPatchDiff(patch: any) {
    const panel = vscode.window.createWebviewPanel(
        'uaidaDiff',
        'UAIDA Patch Diff',
        vscode.ViewColumn.Two,
        { enableScripts: true }
    );

    panel.webview.html = generateDiffHTML(patch);
}

async function showTestResults(results: any) {
    const panel = vscode.window.createWebviewPanel(
        'uaidaTests',
        'UAIDA Test Results',
        vscode.ViewColumn.Two,
        { enableScripts: true }
    );

    panel.webview.html = generateTestResultsHTML(results);
}

async function showReviewResults(review: any) {
    const panel = vscode.window.createWebviewPanel(
        'uaidaReview',
        'UAIDA Code Review',
        vscode.ViewColumn.Two,
        { enableScripts: true }
    );

    panel.webview.html = generateReviewHTML(review);
}

function generatePlanHTML(plan: any): string {
    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UAIDA Plan</title>
    <style>
        body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); }
        .step { margin: 10px 0; padding: 10px; border-left: 3px solid var(--vscode-accent-foreground); }
        .risk-low { color: var(--vscode-testing-iconPassed); }
        .risk-medium { color: var(--vscode-testing-iconQueued); }
        .risk-high { color: var(--vscode-testing-iconFailed); }
    </style>
</head>
<body>
    <h1>Execution Plan</h1>
    <p><strong>Goal:</strong> ${plan.goal}</p>
    <p><strong>Risk Level:</strong> <span class="risk-${plan.risk_level}">${plan.risk_level.toUpperCase()}</span></p>
    <p><strong>Estimated Duration:</strong> ${Math.round(plan.estimated_duration / 60)} minutes</p>
    
    <h2>Steps</h2>
    ${plan.steps.map((step: any, index: number) => `
        <div class="step">
            <h3>${index + 1}. ${step.description}</h3>
            <p><strong>Type:</strong> ${step.step_type}</p>
            <p><strong>Estimated Time:</strong> ${step.estimated_duration} seconds</p>
        </div>
    `).join('')}
    
    <h2>Affected Files</h2>
    <ul>
        ${plan.affected_files.map((file: string) => `<li>${file}</li>`).join('')}
    </ul>
</body>
</html>`;
}

function generateDiffHTML(patch: any): string {
    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UAIDA Patch Diff</title>
    <style>
        body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); }
        .change { margin: 10px 0; padding: 10px; border-radius: 4px; }
        .create { background-color: var(--vscode-diffEditor-insertedTextBackground); }
        .modify { background-color: var(--vscode-diffEditor-modifiedTextBackground); }
        .delete { background-color: var(--vscode-diffEditor-removedTextBackground); }
        .conflict { background-color: var(--vscode-errorForeground); color: white; }
    </style>
</head>
<body>
    <h1>Patch Changes</h1>
    <p><strong>Patch ID:</strong> ${patch.patch_id}</p>
    <p><strong>Success:</strong> ${patch.success ? 'Yes' : 'No'}</p>
    
    <h2>Applied Changes</h2>
    ${patch.changes_applied.map((change: any) => `
        <div class="change ${change.operation.toLowerCase()}">
            <h3>${change.operation}: ${change.file}</h3>
            <p>Lines changed: ${change.lines_changed}</p>
            ${change.backup_path ? `<p>Backup: ${change.backup_path}</p>` : ''}
        </div>
    `).join('')}
    
    ${patch.conflicts.length > 0 ? `
        <h2>Conflicts</h2>
        ${patch.conflicts.map((conflict: string) => `
            <div class="change conflict">
                <p>${conflict}</p>
            </div>
        `).join('')}
    ` : ''}
</body>
</html>`;
}

function generateTestResultsHTML(results: any): string {
    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UAIDA Test Results</title>
    <style>
        body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); }
        .success { color: var(--vscode-testing-iconPassed); }
        .failure { color: var(--vscode-testing-iconFailed); }
        .coverage { margin: 10px 0; padding: 10px; background-color: var(--vscode-editor-background); }
    </style>
</head>
<body>
    <h1>Test Results</h1>
    <p><strong>Run ID:</strong> ${results.run_id}</p>
    <p><strong>Success:</strong> <span class="${results.success ? 'success' : 'failure'}">${results.success ? 'PASSED' : 'FAILED'}</span></p>
    <p><strong>Tests Passed:</strong> ${results.tests_passed}</p>
    <p><strong>Tests Failed:</strong> ${results.tests_failed}</p>
    <p><strong>Execution Time:</strong> ${results.execution_time} ms</p>
    
    ${results.coverage ? `
        <div class="coverage">
            <h2>Coverage</h2>
            <p><strong>Overall:</strong> ${results.coverage.percentage.toFixed(1)}%</p>
            <p><strong>Lines:</strong> ${results.coverage.lines_covered}/${results.coverage.lines_total}</p>
        </div>
    ` : ''}
    
    ${results.failures.length > 0 ? `
        <h2>Failures</h2>
        ${results.failures.map((failure: any) => `
            <div class="failure">
                <h3>${failure.test_name}</h3>
                <p><strong>File:</strong> ${failure.file}:${failure.line}</p>
                <p><strong>Error:</strong> ${failure.error_message}</p>
            </div>
        `).join('')}
    ` : ''}
</body>
</html>`;
}

function generateReviewHTML(review: any): string {
    return `
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UAIDA Risk Report</title>
    <style>
        body { font-family: var(--vscode-font-family); color: var(--vscode-foreground); }
        .risk-low { color: var(--vscode-testing-iconPassed); }
        .risk-medium { color: var(--vscode-testing-iconQueued); }
        .risk-high { color: var(--vscode-testing-iconFailed); }
        .issue { margin: 10px 0; padding: 10px; border-left: 3px solid var(--vscode-errorForeground); }
    </style>
</head>
<body>
    <h1>Risk Assessment Report</h1>
    <p><strong>Patch ID:</strong> ${review.patch_id}</p>
    <p><strong>Risk Level:</strong> <span class="risk-${review.risk_level}">${review.risk_level.toUpperCase()}</span></p>
    <p><strong>Security Score:</strong> ${review.security_score}/10</p>
    <p><strong>Performance Impact:</strong> ${review.performance_impact > 0 ? '+' : ''}${review.performance_impact}%</p>
    
    ${review.security_issues.length > 0 ? `
        <h2>Security Issues</h2>
        ${review.security_issues.map((issue: any) => `
            <div class="issue">
                <h3>${issue.severity.toUpperCase()}: ${issue.description}</h3>
                <p><strong>File:</strong> ${issue.file}:${issue.line}</p>
                <p><strong>Mitigation:</strong> ${issue.mitigation}</p>
            </div>
        `).join('')}
    ` : ''}
    
    <h2>Recommendations</h2>
    <ul>
        ${review.recommendations.map((rec: string) => `<li>${rec}</li>`).join('')}
    </ul>
</body>
</html>`;
}

export function deactivate() {
    if (client) {
        client.dispose();
    }
}
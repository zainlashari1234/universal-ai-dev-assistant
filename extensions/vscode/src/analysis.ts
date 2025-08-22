import * as vscode from 'vscode';
import { UAIDAClient, AnalysisRequest } from './client';

export class AnalysisProvider {
    constructor(private client: UAIDAClient) {}

    async analyzeCode(editor: vscode.TextEditor, analysisType: string) {
        const document = editor.document;
        const selection = editor.selection;
        
        // Get code to analyze
        const code = selection.isEmpty 
            ? document.getText() 
            : document.getText(selection);

        if (!code.trim()) {
            vscode.window.showWarningMessage('No code to analyze');
            return;
        }

        const request: AnalysisRequest = {
            code: code,
            language: document.languageId,
            analysis_type: analysisType
        };

        try {
            await vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: `🔍 Analyzing code for ${analysisType}...`,
                cancellable: true
            }, async (progress, token) => {
                const response = await this.client.analyzeCode(request);
                
                if (token.isCancellationRequested) {
                    return;
                }

                // Show results in a new document
                await this.showAnalysisResults(response, analysisType);
            });
        } catch (error) {
            vscode.window.showErrorMessage(`Analysis failed: ${error}`);
        }
    }

    private async showAnalysisResults(response: any, analysisType: string) {
        const resultsDocument = await vscode.workspace.openTextDocument({
            content: this.formatAnalysisResults(response, analysisType),
            language: 'markdown'
        });

        await vscode.window.showTextDocument(resultsDocument, vscode.ViewColumn.Beside);

        // Show summary notification
        const findingsCount = response.findings?.length || 0;
        const suggestionsCount = response.suggestions?.length || 0;
        
        vscode.window.showInformationMessage(
            `✅ Analysis complete! Found ${findingsCount} issues and ${suggestionsCount} suggestions.`,
            'View Details', 'Apply Fixes'
        ).then(selection => {
            if (selection === 'Apply Fixes') {
                this.showQuickFixes(response);
            }
        });
    }

    private formatAnalysisResults(response: any, analysisType: string): string {
        const timestamp = new Date().toLocaleString();
        const confidence = ((response.confidence_score || 0) * 100).toFixed(1);
        
        let markdown = `# 🔍 UAIDA Code Analysis Report\n\n`;
        markdown += `**Analysis Type:** ${analysisType}\n`;
        markdown += `**Timestamp:** ${timestamp}\n`;
        markdown += `**Provider:** ${response.provider_used}\n`;
        markdown += `**Confidence:** ${confidence}%\n`;
        markdown += `**Response Time:** ${response.response_time_ms}ms\n\n`;

        // Summary
        if (response.summary) {
            markdown += `## 📋 Summary\n\n${response.summary}\n\n`;
        }

        // Findings
        if (response.findings && response.findings.length > 0) {
            markdown += `## 🔍 Findings (${response.findings.length})\n\n`;
            response.findings.forEach((finding: string, index: number) => {
                markdown += `### ${index + 1}. ${this.getIssueIcon(analysisType)} Issue\n\n`;
                markdown += `${finding}\n\n`;
            });
        } else {
            markdown += `## ✅ No Issues Found\n\nGreat! No ${analysisType} issues were detected in your code.\n\n`;
        }

        // Suggestions
        if (response.suggestions && response.suggestions.length > 0) {
            markdown += `## 💡 Suggestions (${response.suggestions.length})\n\n`;
            response.suggestions.forEach((suggestion: string, index: number) => {
                markdown += `### ${index + 1}. Improvement\n\n`;
                markdown += `${suggestion}\n\n`;
            });
        }

        // Analysis details
        markdown += `## 📊 Analysis Details\n\n`;
        markdown += `- **Analysis Type:** ${response.analysis_type}\n`;
        markdown += `- **Provider Used:** ${response.provider_used}\n`;
        markdown += `- **Processing Time:** ${response.response_time_ms}ms\n`;
        markdown += `- **Confidence Score:** ${confidence}%\n\n`;

        // Recommendations
        markdown += `## 🎯 Next Steps\n\n`;
        
        if (response.findings && response.findings.length > 0) {
            markdown += `1. **Review Findings:** Address the ${response.findings.length} issues identified above\n`;
            markdown += `2. **Apply Suggestions:** Consider implementing the recommended improvements\n`;
            markdown += `3. **Re-analyze:** Run the analysis again after making changes\n`;
        } else {
            markdown += `1. **Code Quality:** Your code looks good for ${analysisType}!\n`;
            markdown += `2. **Consider Other Analysis:** Try running other analysis types (security, performance, etc.)\n`;
            markdown += `3. **Continuous Improvement:** Regular analysis helps maintain code quality\n`;
        }

        markdown += `\n---\n\n`;
        markdown += `*Generated by UAIDA - Universal AI Development Assistant*\n`;
        markdown += `*For more information, visit: https://github.com/Tehlikeli107/universal-ai-dev-assistant*`;

        return markdown;
    }

    private getIssueIcon(analysisType: string): string {
        const icons: { [key: string]: string } = {
            'security': '🔒',
            'performance': '⚡',
            'quality': '✨',
            'bugs': '🐛',
            'suggestions': '💡',
            'documentation': '📚',
            'testing': '🧪'
        };
        return icons[analysisType] || '🔍';
    }

    private async showQuickFixes(response: any) {
        if (!response.suggestions || response.suggestions.length === 0) {
            vscode.window.showInformationMessage('No automated fixes available');
            return;
        }

        const quickPick = vscode.window.createQuickPick();
        quickPick.title = 'Select fixes to apply';
        quickPick.canSelectMany = true;
        quickPick.items = response.suggestions.map((suggestion: string, index: number) => ({
            label: `Fix ${index + 1}`,
            description: suggestion.substring(0, 100) + (suggestion.length > 100 ? '...' : ''),
            detail: suggestion
        }));

        quickPick.onDidAccept(() => {
            const selectedFixes = quickPick.selectedItems;
            if (selectedFixes.length > 0) {
                this.applyQuickFixes(selectedFixes.map(item => item.detail));
            }
            quickPick.dispose();
        });

        quickPick.show();
    }

    private async applyQuickFixes(fixes: string[]) {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            vscode.window.showWarningMessage('No active editor to apply fixes');
            return;
        }

        // For now, just show the fixes in a comment
        // In a real implementation, you'd parse the fixes and apply them programmatically
        const fixesComment = `\n// UAIDA Suggested Fixes:\n${fixes.map((fix, i) => `// ${i + 1}. ${fix}`).join('\n')}\n`;
        
        await editor.edit(editBuilder => {
            editBuilder.insert(new vscode.Position(0, 0), fixesComment);
        });

        vscode.window.showInformationMessage(`Applied ${fixes.length} suggested fixes as comments`);
    }

    // Real-time analysis on file save
    async analyzeOnSave(document: vscode.TextDocument) {
        const config = vscode.workspace.getConfiguration('uaida');
        if (!config.get('analyzeOnSave')) {
            return;
        }

        // Only analyze code files
        const codeLanguages = ['typescript', 'javascript', 'python', 'rust', 'java', 'cpp', 'c', 'go'];
        if (!codeLanguages.includes(document.languageId)) {
            return;
        }

        try {
            const request: AnalysisRequest = {
                code: document.getText(),
                language: document.languageId,
                analysis_type: 'quality' // Default to quality analysis
            };

            const response = await this.client.analyzeCode(request);
            
            // Show issues in problems panel
            this.updateProblemsPanel(document, response);
            
        } catch (error) {
            console.error('Auto-analysis failed:', error);
        }
    }

    private updateProblemsPanel(document: vscode.TextDocument, response: any) {
        const diagnostics: vscode.Diagnostic[] = [];
        
        if (response.findings) {
            response.findings.forEach((finding: string, index: number) => {
                const diagnostic = new vscode.Diagnostic(
                    new vscode.Range(0, 0, 0, 0), // Would need better line detection
                    finding,
                    vscode.DiagnosticSeverity.Warning
                );
                diagnostic.source = 'UAIDA';
                diagnostics.push(diagnostic);
            });
        }

        // Create or get diagnostic collection
        const collection = vscode.languages.createDiagnosticCollection('uaida');
        collection.set(document.uri, diagnostics);
    }
}
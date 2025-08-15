import * as vscode from 'vscode';

export class DiffProvider implements vscode.TextDocumentContentProvider {
    private _onDidChange = new vscode.EventEmitter<vscode.Uri>();
    private diffs: Map<string, string> = new Map();

    readonly onDidChange = this._onDidChange.event;

    provideTextDocumentContent(uri: vscode.Uri): string {
        const key = uri.authority || 'latest';
        return this.diffs.get(key) || this.generateSampleDiff();
    }

    updateDiff(key: string, diff: string) {
        this.diffs.set(key, diff);
        const uri = vscode.Uri.parse(`uaida-diff://${key}`);
        this._onDidChange.fire(uri);
    }

    private generateSampleDiff(): string {
        return `--- a/src/math_utils.py
+++ b/src/math_utils.py
@@ -1,5 +1,12 @@
 def divide(a, b):
-    return a / b
+    """Safely divide two numbers with error handling."""
+    if not isinstance(a, (int, float)) or not isinstance(b, (int, float)):
+        raise TypeError("Both arguments must be numbers")
+    
+    if b == 0:
+        raise ValueError("Division by zero is not allowed")
+    
+    return a / b
 
 def multiply(a, b):
     return a * b`;
    }
}
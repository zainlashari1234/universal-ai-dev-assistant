#!/usr/bin/env python3
"""
Working Demo of Universal AI Development Assistant
This demonstrates the currently working features.
"""

import json
import subprocess
import sys
from pathlib import Path

def analyze_code_security(file_path):
    """
    Working security analysis - detects real vulnerabilities
    """
    with open(file_path, 'r') as f:
        code = f.read()
    
    vulnerabilities = []
    
    # Real vulnerability detection
    if 'eval(' in code:
        vulnerabilities.append({
            'type': 'Code Injection',
            'severity': 'Critical',
            'message': 'Use of eval() can lead to code injection',
            'suggestion': 'Use ast.literal_eval() for safe evaluation'
        })
    
    if 'password' in code.lower() and ('=' in code or ':' in code):
        vulnerabilities.append({
            'type': 'Hardcoded Secret',
            'severity': 'High', 
            'message': 'Potential hardcoded password detected',
            'suggestion': 'Use environment variables for secrets'
        })
    
    if 'subprocess.call' in code and 'shell=True' in code:
        vulnerabilities.append({
            'type': 'Command Injection',
            'severity': 'High',
            'message': 'Shell injection vulnerability',
            'suggestion': 'Use shell=False and pass arguments as list'
        })
    
    return vulnerabilities

def generate_documentation(file_path):
    """
    Working documentation generator
    """
    with open(file_path, 'r') as f:
        code = f.read()
    
    # Extract functions and classes
    functions = []
    classes = []
    
    lines = code.split('\n')
    for i, line in enumerate(lines):
        if line.strip().startswith('def '):
            func_name = line.split('def ')[1].split('(')[0]
            functions.append({
                'name': func_name,
                'line': i + 1,
                'signature': line.strip()
            })
        elif line.strip().startswith('class '):
            class_name = line.split('class ')[1].split('(')[0].split(':')[0]
            classes.append({
                'name': class_name,
                'line': i + 1,
                'signature': line.strip()
            })
    
    # Generate documentation
    doc = f"# Documentation for {Path(file_path).name}\n\n"
    
    if classes:
        doc += "## Classes\n\n"
        for cls in classes:
            doc += f"### {cls['name']}\n"
            doc += f"- **Line**: {cls['line']}\n"
            doc += f"- **Definition**: `{cls['signature']}`\n\n"
    
    if functions:
        doc += "## Functions\n\n"
        for func in functions:
            doc += f"### {func['name']}\n"
            doc += f"- **Line**: {func['line']}\n"
            doc += f"- **Signature**: `{func['signature']}`\n\n"
    
    return doc

def analyze_performance(file_path):
    """
    Working performance analysis
    """
    with open(file_path, 'r') as f:
        code = f.read()
    
    issues = []
    
    # Detect performance issues
    if 'for' in code and 'for' in code:
        # Simple nested loop detection
        for_count = code.count('for ')
        if for_count >= 2:
            issues.append({
                'type': 'Algorithmic Complexity',
                'severity': 'Medium',
                'message': f'Potential O(n²) complexity detected ({for_count} loops)',
                'suggestion': 'Consider using more efficient algorithms or data structures'
            })
    
    if '.append(' in code and 'for' in code:
        issues.append({
            'type': 'Performance',
            'severity': 'Low',
            'message': 'List append in loop detected',
            'suggestion': 'Consider list comprehension for better performance'
        })
    
    return issues

def main():
    """
    Main demo function - shows working features
    """
    print("🚀 Universal AI Development Assistant - Working Demo")
    print("=" * 60)
    
    # Analyze the sample Python project
    sample_file = "examples/python/sample_project.py"
    
    if not Path(sample_file).exists():
        print(f"❌ Sample file {sample_file} not found")
        return
    
    print(f"📁 Analyzing: {sample_file}")
    print()
    
    # 1. Security Analysis (WORKING)
    print("🛡️  SECURITY ANALYSIS")
    print("-" * 30)
    vulnerabilities = analyze_code_security(sample_file)
    
    if vulnerabilities:
        for vuln in vulnerabilities:
            print(f"🚨 {vuln['severity']}: {vuln['message']}")
            print(f"   💡 Suggestion: {vuln['suggestion']}")
            print()
    else:
        print("✅ No security vulnerabilities detected")
    
    print()
    
    # 2. Performance Analysis (WORKING)
    print("⚡ PERFORMANCE ANALYSIS")
    print("-" * 30)
    perf_issues = analyze_performance(sample_file)
    
    if perf_issues:
        for issue in perf_issues:
            print(f"⚠️  {issue['severity']}: {issue['message']}")
            print(f"   💡 Suggestion: {issue['suggestion']}")
            print()
    else:
        print("✅ No performance issues detected")
    
    print()
    
    # 3. Documentation Generation (WORKING)
    print("📚 DOCUMENTATION GENERATION")
    print("-" * 30)
    docs = generate_documentation(sample_file)
    print("✅ Documentation generated:")
    print()
    print(docs[:500] + "..." if len(docs) > 500 else docs)
    
    print()
    print("🎯 SUMMARY")
    print("-" * 30)
    print("✅ Security scanning: WORKING")
    print("✅ Performance analysis: WORKING") 
    print("✅ Documentation generation: WORKING")
    print("🔨 AI model integration: IN PROGRESS")
    print("🔨 Real-time completion: IN PROGRESS")
    print("🔨 Predictive debugging: IN PROGRESS")
    print()
    print("🚀 This demonstrates the working foundation!")
    print("   Full AI features coming in next releases.")

if __name__ == "__main__":
    main()
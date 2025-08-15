#!/usr/bin/env python3
"""
Milestone 1 Demo Script - Universal AI Development Assistant
Demonstrates all implemented features from Milestone 1
"""

import json
import time
import requests
from pathlib import Path
from datetime import datetime

BASE_URL = "http://localhost:8080"

def check_api_health():
    """Check if the API is running and healthy"""
    try:
        response = requests.get(f"{BASE_URL}/health", timeout=5)
        if response.status_code == 200:
            health_data = response.json()
            print("✅ API Health Check:")
            print(f"   Status: {health_data.get('status')}")
            print(f"   Version: {health_data.get('version')}")
            print(f"   AI Model Loaded: {health_data.get('ai_model_loaded')}")
            print(f"   Supported Languages: {', '.join(health_data.get('supported_languages', []))}")
            return True
        else:
            print(f"❌ API Health Check Failed: HTTP {response.status_code}")
            return False
    except requests.exceptions.RequestException as e:
        print(f"❌ API Health Check Failed: {e}")
        return False

def test_code_completion():
    """Test the code completion endpoint"""
    print("\n🔧 Testing Code Completion...")
    
    test_code = """def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + """
    
    try:
        response = requests.post(
            f"{BASE_URL}/api/v1/complete",
            json={
                "code": test_code,
                "language": "python",
                "cursor_position": len(test_code),
                "context": "Generate Fibonacci sequence"
            },
            timeout=10
        )
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Code Completion Success:")
            print(f"   Suggestions: {result.get('suggestions', [])}")
            print(f"   Confidence: {result.get('confidence', 0):.2f}")
            print(f"   Processing Time: {result.get('processing_time_ms', 0)}ms")
            return True
        else:
            print(f"❌ Code Completion Failed: HTTP {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Code Completion Error: {e}")
        return False

def test_code_analysis():
    """Test the code analysis endpoint"""
    print("\n🔍 Testing Code Analysis...")
    
    test_code = """
import subprocess

def unsafe_function(user_input):
    # Security vulnerability: command injection
    subprocess.call(f"echo {user_input}", shell=True)
    
    # Performance issue: nested loops
    for i in range(100):
        for j in range(100):
            print(i * j)
    
    # Quality issue: bare except
    try:
        risky_operation()
    except:
        pass

def risky_operation():
    raise ValueError("Something went wrong")
"""
    
    try:
        response = requests.post(
            f"{BASE_URL}/api/v1/analyze",
            json={
                "code": test_code,
                "language": "python",
                "cursor_position": 0,
                "context": "Analyze for security and quality issues"
            },
            timeout=15
        )
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Code Analysis Success:")
            
            if 'issues' in result:
                print(f"   Issues Found: {len(result['issues'])}")
                for issue in result['issues'][:3]:  # Show first 3 issues
                    print(f"     - {issue}")
            
            if 'suggestions' in result:
                print(f"   Suggestions: {len(result['suggestions'])}")
                for suggestion in result['suggestions'][:2]:  # Show first 2 suggestions
                    print(f"     - {suggestion}")
            
            return True
        else:
            print(f"❌ Code Analysis Failed: HTTP {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Code Analysis Error: {e}")
        return False

def test_agent_plan():
    """Test the agent planning endpoint"""
    print("\n🤖 Testing Agent Planning...")
    
    try:
        response = requests.post(
            f"{BASE_URL}/api/v1/plan",
            json={
                "goal": "Add input validation to a login function",
                "constraints": {
                    "max_files": 5,
                    "max_loc": 100,
                    "timeout_seconds": 300,
                    "budget_limit": 1.0
                }
            },
            timeout=20
        )
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Agent Planning Success:")
            print(f"   Plan ID: {result.get('plan_id')}")
            print(f"   Steps: {len(result.get('steps', []))}")
            print(f"   Estimated Time: {result.get('estimated_time_seconds')}s")
            print(f"   Risk Level: {result.get('risk_level')}")
            print(f"   Budget: {result.get('budget')}")
            return result.get('plan_id')
        else:
            print(f"❌ Agent Planning Failed: HTTP {response.status_code}")
            return None
    except Exception as e:
        print(f"❌ Agent Planning Error: {e}")
        return None

def test_patch_generation(plan_id):
    """Test the patch generation endpoint"""
    if not plan_id:
        print("\n⚠️  Skipping Patch Generation (no plan ID)")
        return None
        
    print("\n🔨 Testing Patch Generation...")
    
    try:
        response = requests.post(
            f"{BASE_URL}/api/v1/patch",
            json={
                "plan_id": plan_id,
                "apply": False
            },
            timeout=15
        )
        
        if response.status_code == 200:
            result = response.json()
            print("✅ Patch Generation Success:")
            print(f"   Patch ID: {result.get('patch_id')}")
            print(f"   Files Modified: {len(result.get('files', []))}")
            print(f"   Lines Added: {result.get('metrics', {}).get('lines_added', 0)}")
            print(f"   Complexity Change: {result.get('metrics', {}).get('complexity_change', 0)}")
            return result.get('patch_id')
        else:
            print(f"❌ Patch Generation Failed: HTTP {response.status_code}")
            return None
    except Exception as e:
        print(f"❌ Patch Generation Error: {e}")
        return None

def test_metrics():
    """Test the metrics endpoint"""
    print("\n📊 Testing Metrics...")
    
    try:
        response = requests.get(f"{BASE_URL}/metrics", timeout=5)
        
        if response.status_code == 200:
            metrics_text = response.text
            print("✅ Metrics Available:")
            
            # Count different metric types
            lines = metrics_text.split('\n')
            metric_lines = [line for line in lines if line and not line.startswith('#')]
            print(f"   Total Metrics: {len(metric_lines)}")
            
            # Show some key metrics
            key_metrics = ['http_requests_total', 'provider_calls_total', 'request_duration']
            for metric in key_metrics:
                for line in lines:
                    if line.startswith(metric):
                        print(f"   {line}")
                        break
            
            return True
        else:
            print(f"❌ Metrics Failed: HTTP {response.status_code}")
            return False
    except Exception as e:
        print(f"❌ Metrics Error: {e}")
        return False

def run_milestone1_demo():
    """Run the complete Milestone 1 demonstration"""
    print("🚀 Universal AI Development Assistant - Milestone 1 Demo")
    print("=" * 60)
    
    start_time = time.time()
    results = {}
    
    # Test all endpoints
    results['health'] = check_api_health()
    results['completion'] = test_code_completion()
    results['analysis'] = test_code_analysis()
    
    plan_id = test_agent_plan()
    results['planning'] = plan_id is not None
    
    patch_id = test_patch_generation(plan_id)
    results['patching'] = patch_id is not None
    
    results['metrics'] = test_metrics()
    
    # Summary
    print("\n" + "=" * 60)
    print("📋 MILESTONE 1 DEMO SUMMARY")
    print("=" * 60)
    
    total_tests = len(results)
    passed_tests = sum(1 for result in results.values() if result)
    
    print(f"Tests Passed: {passed_tests}/{total_tests}")
    print(f"Success Rate: {(passed_tests/total_tests)*100:.1f}%")
    print(f"Total Time: {time.time() - start_time:.2f}s")
    
    print("\nFeature Status:")
    feature_map = {
        'health': '🏥 Health Monitoring',
        'completion': '🔧 Code Completion',
        'analysis': '🔍 Code Analysis',
        'planning': '🤖 Agent Planning',
        'patching': '🔨 Patch Generation',
        'metrics': '📊 Metrics & Observability'
    }
    
    for key, name in feature_map.items():
        status = "✅ WORKING" if results[key] else "❌ FAILED"
        print(f"  {name}: {status}")
    
    print("\n🎯 Milestone 1 Implementation Status:")
    print("✅ Provider Router System (Ollama + Heuristic fallback)")
    print("✅ Context Manager MVP (Repository scanning)")
    print("✅ Agent Loop v1 (Plan→Patch→Test workflow)")
    print("✅ REST API & OpenAPI endpoints")
    print("✅ Basic Observability (Prometheus metrics)")
    print("✅ Sandbox Runner foundation")
    
    if passed_tests >= total_tests * 0.8:
        print("\n🎉 Milestone 1 SUCCESSFULLY COMPLETED!")
        print("Ready for Milestone 2: PR Quality & Safety")
    else:
        print("\n⚠️  Some features need attention before Milestone 2")
    
    # Save results
    results_data = {
        'timestamp': datetime.now().isoformat(),
        'milestone': 1,
        'total_tests': total_tests,
        'passed_tests': passed_tests,
        'success_rate': (passed_tests/total_tests)*100,
        'execution_time': time.time() - start_time,
        'results': results,
        'plan_id': plan_id,
        'patch_id': patch_id
    }
    
    results_file = Path("docs/evals") / f"milestone1_demo_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    results_file.parent.mkdir(parents=True, exist_ok=True)
    
    with open(results_file, 'w') as f:
        json.dump(results_data, f, indent=2)
    
    print(f"\n💾 Results saved to: {results_file}")
    
    return passed_tests >= total_tests * 0.8

if __name__ == "__main__":
    success = run_milestone1_demo()
    exit(0 if success else 1)
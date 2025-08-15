#!/usr/bin/env python3
"""
HumanEval+ evaluation runner for Universal AI Development Assistant
"""

import json
import time
import argparse
import requests
from pathlib import Path
from typing import Dict, List, Any
from datetime import datetime
import subprocess
import tempfile
import os

def load_humaneval_plus(dataset_path: str) -> List[Dict[str, Any]]:
    """Load HumanEval+ dataset"""
    problems = []
    with open(dataset_path, 'r') as f:
        for line in f:
            problems.append(json.loads(line.strip()))
    return problems

def call_completion_api(prompt: str, model: str = "ollama-qwen") -> str:
    """Call the UAIDA completion API"""
    try:
        response = requests.post(
            "http://localhost:8080/api/v1/complete",
            json={
                "code": prompt,
                "language": "python",
                "cursor_position": len(prompt),
                "context": None
            },
            timeout=30
        )
        
        if response.status_code == 200:
            result = response.json()
            suggestions = result.get("suggestions", [])
            return suggestions[0] if suggestions else ""
        else:
            print(f"API error: {response.status_code}")
            return ""
    except Exception as e:
        print(f"API call failed: {e}")
        return ""

def execute_code(code: str) -> Dict[str, Any]:
    """Execute Python code safely and return results"""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(code)
        temp_file = f.name
    
    try:
        result = subprocess.run(
            ['python', temp_file],
            capture_output=True,
            text=True,
            timeout=10
        )
        
        return {
            "success": result.returncode == 0,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "returncode": result.returncode
        }
    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "stdout": "",
            "stderr": "Execution timeout",
            "returncode": -1
        }
    except Exception as e:
        return {
            "success": False,
            "stdout": "",
            "stderr": str(e),
            "returncode": -1
        }
    finally:
        os.unlink(temp_file)

def evaluate_solution(problem: Dict[str, Any], completion: str) -> Dict[str, Any]:
    """Evaluate a single solution"""
    task_id = problem["task_id"]
    prompt = problem["prompt"]
    tests = problem.get("test", "")
    plus_tests = problem.get("plus_test", "")
    
    # Combine prompt and completion
    full_code = prompt + completion
    
    # Add tests
    test_code = full_code + "\n\n" + tests
    if plus_tests:
        test_code += "\n\n" + plus_tests
    
    # Execute and check
    result = execute_code(test_code)
    
    return {
        "task_id": task_id,
        "completion": completion,
        "passed": result["success"],
        "result": result
    }

def run_evaluation(dataset_path: str, model: str, max_problems: int = None) -> Dict[str, Any]:
    """Run full HumanEval+ evaluation"""
    print(f"🚀 Starting HumanEval+ evaluation with model: {model}")
    
    # Load dataset
    problems = load_humaneval_plus(dataset_path)
    if max_problems:
        problems = problems[:max_problems]
    
    print(f"📊 Evaluating {len(problems)} problems...")
    
    results = []
    passed = 0
    
    start_time = time.time()
    
    for i, problem in enumerate(problems):
        print(f"Problem {i+1}/{len(problems)}: {problem['task_id']}")
        
        # Get completion from API
        completion = call_completion_api(problem["prompt"], model)
        
        if not completion:
            print(f"  ❌ No completion generated")
            results.append({
                "task_id": problem["task_id"],
                "completion": "",
                "passed": False,
                "result": {"success": False, "stderr": "No completion"}
            })
            continue
        
        # Evaluate solution
        eval_result = evaluate_solution(problem, completion)
        results.append(eval_result)
        
        if eval_result["passed"]:
            passed += 1
            print(f"  ✅ Passed")
        else:
            print(f"  ❌ Failed: {eval_result['result']['stderr'][:100]}")
    
    end_time = time.time()
    
    # Calculate metrics
    total_problems = len(problems)
    pass_rate = (passed / total_problems) * 100 if total_problems > 0 else 0
    avg_time = (end_time - start_time) / total_problems if total_problems > 0 else 0
    
    summary = {
        "model": model,
        "dataset": "HumanEval+",
        "total_problems": total_problems,
        "passed": passed,
        "pass_rate": pass_rate,
        "avg_time_per_problem": avg_time,
        "total_time": end_time - start_time,
        "timestamp": datetime.now().isoformat(),
        "results": results
    }
    
    return summary

def save_results(results: Dict[str, Any], output_dir: str):
    """Save evaluation results"""
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"humaneval_plus_{results['model']}_{timestamp}.json"
    
    with open(output_path / filename, 'w') as f:
        json.dump(results, f, indent=2)
    
    print(f"💾 Results saved to {output_path / filename}")
    
    # Also save a summary
    summary_file = output_path / "latest_summary.txt"
    with open(summary_file, 'w') as f:
        f.write(f"HumanEval+ Evaluation Results\n")
        f.write(f"============================\n\n")
        f.write(f"Model: {results['model']}\n")
        f.write(f"Date: {results['timestamp']}\n")
        f.write(f"Total Problems: {results['total_problems']}\n")
        f.write(f"Passed: {results['passed']}\n")
        f.write(f"Pass Rate: {results['pass_rate']:.2f}%\n")
        f.write(f"Average Time: {results['avg_time_per_problem']:.2f}s\n")
        f.write(f"Total Time: {results['total_time']:.2f}s\n")

def main():
    parser = argparse.ArgumentParser(description="Run HumanEval+ evaluation")
    parser.add_argument("--model", default="ollama-qwen", help="Model to evaluate")
    parser.add_argument("--dataset", default="./data/evals/humaneval_plus.jsonl", help="Dataset path")
    parser.add_argument("--output", default="./docs/evals", help="Output directory")
    parser.add_argument("--max-problems", type=int, help="Maximum number of problems to evaluate")
    parser.add_argument("--download", action="store_true", help="Download dataset if missing")
    
    args = parser.parse_args()
    
    # Check if dataset exists
    if not Path(args.dataset).exists():
        if args.download:
            print("📥 Dataset not found, downloading...")
            subprocess.run(["bash", "scripts/evals/download_datasets.sh"], check=True)
        else:
            print(f"❌ Dataset not found: {args.dataset}")
            print("Run with --download to download datasets automatically")
            return 1
    
    # Check if API is running
    try:
        response = requests.get("http://localhost:8080/health", timeout=5)
        if response.status_code != 200:
            print("❌ UAIDA API is not responding. Start the backend server first.")
            return 1
    except requests.exceptions.RequestException:
        print("❌ UAIDA API is not running. Start the backend server first:")
        print("   cd backend && cargo run")
        return 1
    
    # Run evaluation
    results = run_evaluation(args.dataset, args.model, args.max_problems)
    
    # Save results
    save_results(results, args.output)
    
    # Print summary
    print(f"\n🎯 Evaluation Complete!")
    print(f"Model: {results['model']}")
    print(f"Pass Rate: {results['pass_rate']:.2f}% ({results['passed']}/{results['total_problems']})")
    print(f"Average Time: {results['avg_time_per_problem']:.2f}s per problem")
    
    return 0

if __name__ == "__main__":
    exit(main())
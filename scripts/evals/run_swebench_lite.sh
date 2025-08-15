#!/bin/bash
# SWE-bench Lite evaluation runner

set -e

MODEL=${1:-"ollama-qwen"}
DATASET_PATH="./data/evals/swe_bench_lite.jsonl"
OUTPUT_DIR="./docs/evals"
MAX_PROBLEMS=${MAX_PROBLEMS:-10}  # Limit for MVP

echo "🚀 Starting SWE-bench Lite evaluation with model: $MODEL"

# Check if dataset exists
if [ ! -f "$DATASET_PATH" ]; then
    echo "📥 Dataset not found, downloading..."
    bash scripts/evals/download_datasets.sh
fi

# Check if API is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    echo "❌ UAIDA API is not running. Start the backend server first:"
    echo "   cd backend && cargo run"
    exit 1
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Generate timestamp
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RESULTS_FILE="$OUTPUT_DIR/swebench_lite_${MODEL}_${TIMESTAMP}.json"

echo "📊 Evaluating up to $MAX_PROBLEMS problems..."

# Python script for SWE-bench evaluation
python3 << EOF
import json
import requests
import time
from datetime import datetime
from pathlib import Path

def load_swebench_problems(dataset_path, max_problems=None):
    """Load SWE-bench problems"""
    problems = []
    with open(dataset_path, 'r') as f:
        for i, line in enumerate(f):
            if max_problems and i >= max_problems:
                break
            problems.append(json.loads(line.strip()))
    return problems

def call_plan_api(problem_statement, repo_info):
    """Call the UAIDA plan API"""
    try:
        response = requests.post(
            "http://localhost:8080/api/v1/plan",
            json={
                "goal": problem_statement,
                "repo": repo_info,
                "constraints": {
                    "max_files": 5,
                    "max_loc": 100,
                    "timeout_s": 300
                }
            },
            timeout=60
        )
        
        if response.status_code == 200:
            return response.json()
        else:
            print(f"Plan API error: {response.status_code}")
            return None
    except Exception as e:
        print(f"Plan API call failed: {e}")
        return None

def call_patch_api(plan_id):
    """Call the UAIDA patch API"""
    try:
        response = requests.post(
            "http://localhost:8080/api/v1/patch",
            json={
                "plan_id": plan_id,
                "apply": False  # Just generate, don't apply
            },
            timeout=120
        )
        
        if response.status_code == 200:
            return response.json()
        else:
            print(f"Patch API error: {response.status_code}")
            return None
    except Exception as e:
        print(f"Patch API call failed: {e}")
        return None

def evaluate_swebench_problem(problem):
    """Evaluate a single SWE-bench problem"""
    instance_id = problem["instance_id"]
    problem_statement = problem["problem_statement"]
    repo = problem["repo"]
    
    print(f"Evaluating {instance_id}...")
    
    start_time = time.time()
    
    # Step 1: Generate plan
    plan_result = call_plan_api(problem_statement, repo)
    if not plan_result:
        return {
            "instance_id": instance_id,
            "success": False,
            "error": "Failed to generate plan",
            "time": time.time() - start_time
        }
    
    plan_id = plan_result.get("plan_id")
    if not plan_id:
        return {
            "instance_id": instance_id,
            "success": False,
            "error": "No plan ID returned",
            "time": time.time() - start_time
        }
    
    # Step 2: Generate patch
    patch_result = call_patch_api(plan_id)
    if not patch_result:
        return {
            "instance_id": instance_id,
            "success": False,
            "error": "Failed to generate patch",
            "time": time.time() - start_time,
            "plan": plan_result
        }
    
    # For MVP, we consider it successful if we generated a patch
    # In full implementation, we would apply and test the patch
    patch_generated = bool(patch_result.get("diff"))
    
    return {
        "instance_id": instance_id,
        "success": patch_generated,
        "time": time.time() - start_time,
        "plan": plan_result,
        "patch": patch_result,
        "error": None if patch_generated else "No patch generated"
    }

def main():
    # Load problems
    problems = load_swebench_problems("$DATASET_PATH", $MAX_PROBLEMS)
    print(f"Loaded {len(problems)} problems")
    
    results = []
    successful = 0
    total_time = 0
    
    for i, problem in enumerate(problems):
        print(f"Problem {i+1}/{len(problems)}: {problem['instance_id']}")
        
        result = evaluate_swebench_problem(problem)
        results.append(result)
        
        if result["success"]:
            successful += 1
            print(f"  ✅ Success")
        else:
            print(f"  ❌ Failed: {result['error']}")
        
        total_time += result["time"]
        print(f"  ⏱️  Time: {result['time']:.2f}s")
    
    # Calculate metrics
    success_rate = (successful / len(problems)) * 100 if problems else 0
    avg_time = total_time / len(problems) if problems else 0
    
    summary = {
        "model": "$MODEL",
        "dataset": "SWE-bench Lite",
        "total_problems": len(problems),
        "successful": successful,
        "success_rate": success_rate,
        "avg_time_per_problem": avg_time,
        "total_time": total_time,
        "timestamp": datetime.now().isoformat(),
        "results": results
    }
    
    # Save results
    with open("$RESULTS_FILE", 'w') as f:
        json.dump(summary, f, indent=2)
    
    print(f"\n🎯 SWE-bench Lite Evaluation Complete!")
    print(f"Model: {summary['model']}")
    print(f"Success Rate: {summary['success_rate']:.2f}% ({summary['successful']}/{summary['total_problems']})")
    print(f"Average Time: {summary['avg_time_per_problem']:.2f}s per problem")
    print(f"Results saved to: $RESULTS_FILE")

if __name__ == "__main__":
    main()
EOF

echo "✅ SWE-bench Lite evaluation completed!"
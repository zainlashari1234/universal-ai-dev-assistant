#!/bin/bash
# Download evaluation datasets

set -e

DATASETS_DIR="./data/evals"
mkdir -p "$DATASETS_DIR"

echo "📥 Downloading evaluation datasets..."

# HumanEval+
echo "Downloading HumanEval+..."
if [ ! -f "$DATASETS_DIR/humaneval_plus.jsonl" ]; then
    curl -L "https://github.com/evalplus/evalplus/releases/download/v0.2.0/HumanEvalPlus-OriginFmt.jsonl.gz" \
        | gunzip > "$DATASETS_DIR/humaneval_plus.jsonl"
    echo "✅ HumanEval+ downloaded"
else
    echo "✅ HumanEval+ already exists"
fi

# SWE-bench Lite
echo "Downloading SWE-bench Lite..."
if [ ! -f "$DATASETS_DIR/swe_bench_lite.jsonl" ]; then
    curl -L "https://github.com/princeton-nlp/SWE-bench/releases/download/v1.0.0/swe-bench-lite.jsonl" \
        -o "$DATASETS_DIR/swe_bench_lite.jsonl"
    echo "✅ SWE-bench Lite downloaded"
else
    echo "✅ SWE-bench Lite already exists"
fi

# CRUXEval
echo "Downloading CRUXEval..."
if [ ! -f "$DATASETS_DIR/cruxeval.jsonl" ]; then
    curl -L "https://github.com/facebookresearch/cruxeval/releases/download/v1.0.0/cruxeval.jsonl" \
        -o "$DATASETS_DIR/cruxeval.jsonl"
    echo "✅ CRUXEval downloaded"
else
    echo "✅ CRUXEval already exists"
fi

echo "📊 Dataset summary:"
echo "- HumanEval+: $(wc -l < "$DATASETS_DIR/humaneval_plus.jsonl") problems"
echo "- SWE-bench Lite: $(wc -l < "$DATASETS_DIR/swe_bench_lite.jsonl") problems"
echo "- CRUXEval: $(wc -l < "$DATASETS_DIR/cruxeval.jsonl") problems"
echo ""
echo "✅ All datasets downloaded to $DATASETS_DIR"
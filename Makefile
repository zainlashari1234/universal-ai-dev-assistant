# P0 Day-5: Makefile with evaluation commands
.PHONY: help build test run clean bench evals docker dev-setup

# Default target
help:
	@echo "Universal AI Development Assistant - Build Commands"
	@echo "=================================================="
	@echo ""
	@echo "Development:"
	@echo "  dev-setup    Set up development environment"
	@echo "  build        Build the application"
	@echo "  test         Run all tests"
	@echo "  run          Run the development server"
	@echo "  clean        Clean build artifacts"
	@echo ""
	@echo "Evaluation:"
	@echo "  bench        Run all evaluation suites (alias: evals)"
	@echo "  evals        Run evaluation suites and publish results"
	@echo "  evals-html   Generate HTML reports from existing results"
	@echo "  evals-clean  Clean evaluation outputs"
	@echo ""
	@echo "Deployment:"
	@echo "  docker       Build Docker image"
	@echo "  docker-run   Run Docker container"
	@echo ""

# P0 Day-5: Evaluation commands
bench: evals

evals:
	@echo "🎯 Running evaluation suites..."
	@echo "This will run HumanEval+, SWE-bench Lite, and Code Completion evaluations"
	@cargo run --bin eval_runner --features evaluations
	@echo "✅ Evaluations completed - check docs/evals/ for results"

evals-html:
	@echo "📄 Generating HTML reports..."
	@cargo run --bin eval_publisher --features evaluations -- --html-only
	@echo "✅ HTML reports generated"

evals-clean:
	@echo "🧹 Cleaning evaluation outputs..."
	@rm -rf docs/evals/*/
	@echo "✅ Evaluation outputs cleaned"

# Build application
build:
	@echo "🔨 Building application..."
	@cargo build --release
	@echo "✅ Build completed"

# Run tests
test:
	@echo "🧪 Running tests..."
	@cargo test --all-features
	@echo "✅ Tests completed"
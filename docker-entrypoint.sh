#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Universal AI Development Assistant${NC}"
echo -e "${BLUE}======================================${NC}"

# Function to print colored output
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Initialize Ollama in background
start_ollama() {
    log_info "Starting Ollama service..."
    ollama serve &
    OLLAMA_PID=$!
    
    # Wait for Ollama to be ready
    for i in {1..30}; do
        if curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
            log_success "Ollama is ready!"
            break
        fi
        if [ $i -eq 30 ]; then
            log_warning "Ollama failed to start, continuing without it"
            return 1
        fi
        sleep 2
    done
    
    # Pull default models
    log_info "Pulling CodeLlama model..."
    ollama pull codellama:7b-instruct 2>/dev/null || log_warning "Failed to pull CodeLlama model"
    
    log_info "Pulling CodeT5 model..."
    ollama pull codet5:base 2>/dev/null || log_warning "Failed to pull CodeT5 model"
}

# Start frontend if available
start_frontend() {
    if [ -d "/app/frontend" ] && [ -f "/app/frontend/package.json" ]; then
        log_info "Starting frontend development server..."
        cd /app/frontend
        npm start &
        FRONTEND_PID=$!
        cd /app
        log_success "Frontend started on port 3000"
    else
        log_info "No frontend found, skipping..."
    fi
}

# Start the main application
start_server() {
    log_info "Starting Universal AI Development Assistant server..."
    
    # Set environment variables
    export RUST_LOG=${RUST_LOG:-info}
    export UNIVERSAL_AI_CONFIG=${UNIVERSAL_AI_CONFIG:-/app/config.toml}
    export UNIVERSAL_AI_PORT=${UNIVERSAL_AI_PORT:-8080}
    export UNIVERSAL_AI_HOST=${UNIVERSAL_AI_HOST:-0.0.0.0}
    
    # Start the server
    exec /app/universal-ai-dev-assistant
}

# Run demo
run_demo() {
    log_info "Running Universal AI Development Assistant demo..."
    python3 /app/examples/working_demo.py
}

# Run tests
run_tests() {
    log_info "Running comprehensive tests..."
    if [ -f "/app/scripts/build_and_test.sh" ]; then
        bash /app/scripts/build_and_test.sh
    else
        log_warning "Test script not found"
    fi
}

# Setup development environment
setup_dev() {
    log_info "Setting up development environment..."
    
    # Install additional development tools
    log_info "Installing development dependencies..."
    
    # Start all services
    start_ollama
    start_frontend
    
    log_success "Development environment ready!"
    log_info "Services:"
    log_info "  - API Server: http://localhost:8080"
    log_info "  - Frontend: http://localhost:3000"
    log_info "  - Ollama: http://localhost:11434"
    log_info "  - Health Check: http://localhost:8080/health"
    
    # Keep container running
    tail -f /dev/null
}

# Cleanup function
cleanup() {
    log_info "Shutting down services..."
    [ ! -z "$OLLAMA_PID" ] && kill $OLLAMA_PID 2>/dev/null || true
    [ ! -z "$FRONTEND_PID" ] && kill $FRONTEND_PID 2>/dev/null || true
    log_success "Cleanup completed"
}

# Set trap for cleanup
trap cleanup EXIT

# Main command handling
case "$1" in
    "server")
        start_ollama
        start_server
        ;;
    "demo")
        run_demo
        ;;
    "test")
        run_tests
        ;;
    "dev")
        setup_dev
        ;;
    "ollama")
        start_ollama
        tail -f /dev/null
        ;;
    "frontend")
        start_frontend
        tail -f /dev/null
        ;;
    "bash")
        exec /bin/bash
        ;;
    *)
        log_info "Universal AI Development Assistant Docker Container"
        log_info ""
        log_info "Available commands:"
        log_info "  server  - Start the AI assistant server (default)"
        log_info "  demo    - Run the working demo"
        log_info "  test    - Run comprehensive tests"
        log_info "  dev     - Start development environment (all services)"
        log_info "  ollama  - Start only Ollama service"
        log_info "  frontend- Start only frontend"
        log_info "  bash    - Open bash shell"
        log_info ""
        log_info "Examples:"
        log_info "  docker run -p 8080:8080 universal-ai server"
        log_info "  docker run -p 8080:8080 -p 3000:3000 universal-ai dev"
        log_info "  docker run universal-ai demo"
        exit 1
        ;;
esac
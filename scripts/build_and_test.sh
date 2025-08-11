#!/bin/bash

# Universal AI Development Assistant - Build and Test Script
# This script builds the project and runs comprehensive tests

set -e

echo "🚀 Universal AI Development Assistant - Build & Test"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [ ! -f "backend/Cargo.toml" ]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

print_status "Checking prerequisites..."

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check Python installation
if ! command -v python3 &> /dev/null; then
    print_error "Python 3 not found. Please install Python 3"
    exit 1
fi

print_success "Prerequisites check passed"

# Build the backend
print_status "Building Rust backend..."
cd backend

if cargo build --release; then
    print_success "Backend build completed"
else
    print_error "Backend build failed"
    exit 1
fi

# Run backend tests
print_status "Running backend tests..."
if cargo test; then
    print_success "Backend tests passed"
else
    print_warning "Some backend tests failed"
fi

cd ..

# Test Python demo
print_status "Testing Python demo..."
if python3 examples/working_demo.py; then
    print_success "Python demo completed successfully"
else
    print_warning "Python demo had issues"
fi

# Check if Ollama is available
print_status "Checking Ollama availability..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    print_success "Ollama is running and available"
    print_status "Available models:"
    curl -s http://localhost:11434/api/tags | python3 -m json.tool 2>/dev/null || echo "Could not parse models"
else
    print_warning "Ollama not running. Install with: curl -fsSL https://ollama.ai/install.sh | sh"
    print_warning "Then run: ollama pull codellama:7b-instruct"
fi

# Start the server in background for testing
print_status "Starting API server for integration test..."
cd backend
cargo run --release &
SERVER_PID=$!
cd ..

# Wait for server to start
sleep 5

# Test API endpoints
print_status "Testing API endpoints..."
if curl -s http://localhost:8080/health > /dev/null; then
    print_success "API server is responding"
    
    # Test health endpoint
    print_status "Health check response:"
    curl -s http://localhost:8080/health | python3 -m json.tool
    
    # Test completion endpoint
    print_status "Testing completion endpoint..."
    curl -s -X POST http://localhost:8080/api/v1/complete \
        -H "Content-Type: application/json" \
        -d '{"code":"def hello(", "language":"python", "cursor_position":10}' \
        | python3 -m json.tool
        
else
    print_warning "API server not responding"
fi

# Cleanup
print_status "Cleaning up..."
kill $SERVER_PID 2>/dev/null || true

print_success "Build and test completed!"
echo ""
echo "🎯 Next Steps:"
echo "1. Start the server: cd backend && cargo run"
echo "2. Test the API: python3 examples/working_demo.py"
echo "3. Install Ollama for AI features: https://ollama.ai/"
echo "4. Pull a model: ollama pull codellama:7b-instruct"
echo ""
echo "📚 Documentation: README.md"
echo "🐛 Issues: https://github.com/your-repo/issues"
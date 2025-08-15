#!/bin/bash

# Universal AI Development Assistant - Run All Services
# This script starts all UAIDA services in development mode

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Check if we're in the right directory
if [ ! -f "README.md" ] || [ ! -d "backend" ] || [ ! -d "frontend" ]; then
    print_error "Please run this script from the UAIDA root directory"
    exit 1
fi

echo "🚀 Universal AI Development Assistant v6.2.0"
echo "============================================="
echo

# Function to kill background processes on exit
cleanup() {
    print_info "Shutting down services..."
    jobs -p | xargs -r kill
    exit 0
}

trap cleanup SIGINT SIGTERM

# Check dependencies
check_dependency() {
    if command -v $1 &> /dev/null; then
        return 0
    else
        print_error "$1 is not installed"
        return 1
    fi
}

print_info "Checking dependencies..."

if ! check_dependency "cargo"; then
    print_error "Rust/Cargo is required. Please install Rust first."
    exit 1
fi

if ! check_dependency "node"; then
    print_error "Node.js is required. Please install Node.js first."
    exit 1
fi

if ! check_dependency "npm"; then
    print_error "npm is required. Please install npm first."
    exit 1
fi

# Load environment variables
if [ -f ".env" ]; then
    print_info "Loading environment variables from .env"
    export $(cat .env | grep -v '^#' | xargs)
else
    print_warning ".env file not found. Using defaults."
    print_info "Run 'cp .env.example .env' and configure your API keys"
fi

# Start backend
print_info "Starting backend server..."
cd backend
cargo build --release
RUST_LOG=info cargo run --release &
BACKEND_PID=$!
cd ..

# Wait for backend to start
print_info "Waiting for backend to start..."
sleep 5

# Check if backend is running
if ! curl -s http://localhost:8080/health > /dev/null; then
    print_warning "Backend might not be ready yet. Continuing anyway..."
else
    print_success "Backend is running on http://localhost:8080"
fi

# Start frontend
print_info "Starting frontend..."
cd frontend
if [ ! -d "node_modules" ]; then
    print_info "Installing frontend dependencies..."
    npm install
fi

npm start &
FRONTEND_PID=$!
cd ..

# Wait for frontend to start
print_info "Waiting for frontend to start..."
sleep 10

if curl -s http://localhost:3000 > /dev/null; then
    print_success "Frontend is running on http://localhost:3000"
else
    print_warning "Frontend might not be ready yet"
fi

# Show status
echo
print_success "🎉 All services started successfully!"
echo
print_info "📊 Service Status:"
echo "  • Backend:  http://localhost:8080"
echo "  • Frontend: http://localhost:3000"
echo "  • Health:   http://localhost:8080/health"
echo "  • API Docs: http://localhost:8080/api/v1/"
echo
print_info "🔧 Available Commands:"
echo "  • CLI: uaida --help"
echo "  • Status: uaida status"
echo "  • Chat: uaida chat"
echo "  • Dev Mode: uaida dev"
echo
print_info "📚 Quick Start:"
echo "  1. Open http://localhost:3000 in your browser"
echo "  2. Configure your AI providers in the settings"
echo "  3. Start coding with AI assistance!"
echo
print_info "🛑 To stop all services, press Ctrl+C"
echo

# Keep script running and show logs
print_info "Monitoring services... (Press Ctrl+C to stop)"

# Function to check service health
check_services() {
    while true; do
        sleep 30
        
        # Check backend
        if ! curl -s http://localhost:8080/health > /dev/null; then
            print_warning "Backend health check failed"
        fi
        
        # Check frontend
        if ! curl -s http://localhost:3000 > /dev/null; then
            print_warning "Frontend health check failed"
        fi
    done
}

# Start health monitoring in background
check_services &

# Wait for user to stop
wait
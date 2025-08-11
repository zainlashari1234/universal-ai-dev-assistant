# Universal AI Development Assistant - Makefile

.PHONY: help install build test run clean dev docker-build docker-run

# Default target
help:
	@echo "Universal AI Development Assistant"
	@echo ""
	@echo "Available commands:"
	@echo "  install      - Install all dependencies"
	@echo "  build        - Build all components"
	@echo "  test         - Run all tests"
	@echo "  run          - Run the development server"
	@echo "  dev          - Start development environment"
	@echo "  clean        - Clean build artifacts"
	@echo "  docker-build - Build Docker images"
	@echo "  docker-run   - Run with Docker Compose"
	@echo "  extension    - Build VS Code extension"
	@echo "  format       - Format code"
	@echo "  lint         - Run linters"

# Installation
install: install-backend install-frontend install-extension
	@echo "✅ All dependencies installed"

install-backend:
	@echo "📦 Installing backend dependencies..."
	cd backend && cargo build

install-frontend:
	@echo "📦 Installing frontend dependencies..."
	cd frontend && npm install

install-extension:
	@echo "📦 Installing VS Code extension dependencies..."
	cd extensions/vscode && npm install

# Building
build: build-backend build-frontend build-extension
	@echo "✅ All components built"

build-backend:
	@echo "🔨 Building backend..."
	cd backend && cargo build --release

build-frontend:
	@echo "🔨 Building frontend..."
	cd frontend && npm run build

build-extension:
	@echo "🔨 Building VS Code extension..."
	cd extensions/vscode && npm run compile

# Testing
test: test-backend test-frontend
	@echo "✅ All tests completed"

test-backend:
	@echo "🧪 Running backend tests..."
	cd backend && cargo test

test-frontend:
	@echo "🧪 Running frontend tests..."
	cd frontend && npm test -- --watchAll=false

# Development
dev:
	@echo "🚀 Starting development environment..."
	@echo "Starting backend server..."
	cd backend && cargo run &
	@echo "Starting frontend development server..."
	cd frontend && npm start &
	@echo "Development servers started!"

run: build-backend
	@echo "🚀 Starting production server..."
	cd backend && ./target/release/universal-ai-dev-assistant

# Docker
docker-build:
	@echo "🐳 Building Docker images..."
	docker-compose build

docker-run:
	@echo "🐳 Running with Docker Compose..."
	docker-compose up

# VS Code Extension
extension: build-extension
	@echo "📦 Packaging VS Code extension..."
	cd extensions/vscode && npm run package

extension-install: extension
	@echo "📦 Installing VS Code extension locally..."
	code --install-extension extensions/vscode/*.vsix

# Code quality
format:
	@echo "🎨 Formatting code..."
	cd backend && cargo fmt
	cd frontend && npm run format || true
	cd extensions/vscode && npm run format || true

lint:
	@echo "🔍 Running linters..."
	cd backend && cargo clippy
	cd frontend && npm run lint || true
	cd extensions/vscode && npm run lint

# Cleanup
clean:
	@echo "🧹 Cleaning build artifacts..."
	cd backend && cargo clean
	cd frontend && rm -rf build node_modules
	cd extensions/vscode && rm -rf out node_modules *.vsix
	docker-compose down --volumes --remove-orphans || true

# Setup development environment
setup: install
	@echo "⚙️  Setting up development environment..."
	@echo "Creating configuration files..."
	cd backend && cargo run -- --init-config || true
	@echo "✅ Development environment ready!"
	@echo ""
	@echo "Next steps:"
	@echo "1. Run 'make dev' to start development servers"
	@echo "2. Open http://localhost:3000 for the web interface"
	@echo "3. Install the VS Code extension with 'make extension-install'"

# Release
release: test build
	@echo "🚀 Creating release..."
	cd backend && cargo build --release
	cd frontend && npm run build
	cd extensions/vscode && npm run package
	@echo "✅ Release artifacts created"

# Documentation
docs:
	@echo "📚 Generating documentation..."
	cd backend && cargo doc --no-deps
	@echo "📚 Backend documentation generated at backend/target/doc/"

# Database setup (for future use)
db-setup:
	@echo "🗄️  Setting up database..."
	cd backend && sqlx database create || true
	cd backend && sqlx migrate run || true

# Quick start for new developers
quickstart: setup
	@echo "🎉 Quick start complete!"
	@echo ""
	@echo "🔥 Your Universal AI Development Assistant is ready!"
	@echo ""
	@echo "Run 'make dev' to start coding!"
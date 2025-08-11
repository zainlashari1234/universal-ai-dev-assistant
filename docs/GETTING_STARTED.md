# Getting Started with Universal AI Development Assistant

Welcome to UAIDA! This guide will help you get up and running quickly.

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** (for backend)
- **Node.js 18+** (for frontend and VS Code extension)
- **VS Code** (recommended IDE)
- **Docker** (optional, for containerized deployment)

### Installation

#### Option 1: One-Command Setup (Recommended)

```bash
git clone https://github.com/yourusername/universal-ai-dev-assistant
cd universal-ai-dev-assistant
make setup
```

#### Option 2: Manual Setup

```bash
# Clone repository
git clone https://github.com/yourusername/universal-ai-dev-assistant
cd universal-ai-dev-assistant

# Install backend dependencies
cd backend && cargo build && cd ..

# Install frontend dependencies  
cd frontend && npm install && cd ..

# Install VS Code extension dependencies
cd extensions/vscode && npm install && cd ../..
```

### First Run

1. **Start the development environment:**
   ```bash
   make dev
   ```

2. **Open your browser to:**
   - Frontend: http://localhost:3000
   - API: http://localhost:8080/health

3. **Install VS Code extension:**
   ```bash
   make extension-install
   ```

## 🎯 Basic Usage

### VS Code Extension

1. **Open any code file** in VS Code
2. **Start typing** - AI suggestions will appear automatically
3. **Use keyboard shortcuts:**
   - `Ctrl+Space` - Manual completion
   - `Ctrl+Shift+A` - Analyze code

### Web Interface

1. **Open http://localhost:3000**
2. **View dashboard** with server status and metrics
3. **Use playground** to test AI features

### CLI Tool

```bash
# Analyze a file
uaida analyze src/main.py

# Get code completion
uaida complete src/main.py --line 42

# Generate documentation
uaida docs src/ --output docs/

# Generate tests
uaida test src/main.py --framework pytest
```

## 📖 Core Features

### 1. Code Completion

**Automatic suggestions** as you type:
```python
def fibonacci(n):
    if n <= 1:
        return n
    return  # AI suggests: fibonacci(n-1) + fibonacci(n-2)
```

### 2. Code Analysis

**Security and quality analysis:**
```python
password = "admin123"  # ⚠️ Hardcoded password detected
eval(user_input)       # 🚨 Code injection vulnerability
```

### 3. Performance Optimization

**Efficiency suggestions:**
```python
# Before (O(n²))
for i in range(len(items)):
    for j in range(len(items)):
        if items[i] == target:
            return i

# AI suggests: Use dict for O(1) lookup
```

### 4. Documentation Generation

**Auto-generate docstrings:**
```python
def calculate_distance(x1, y1, x2, y2):
    # AI generates:
    """
    Calculate Euclidean distance between two points.
    
    Args:
        x1, y1: Coordinates of first point
        x2, y2: Coordinates of second point
        
    Returns:
        float: Distance between points
    """
```

## ⚙️ Configuration

### Server Configuration

Edit `config.toml`:
```toml
[server]
host = "127.0.0.1"
port = 8080

[ai]
model_name = "codellama-7b-instruct"
max_tokens = 2048
temperature = 0.1

[database]
url = "sqlite://./data/uaida.db"
```

### VS Code Extension Settings

```json
{
    "uaida.serverUrl": "http://127.0.0.1:8080",
    "uaida.enableAutoComplete": true,
    "uaida.maxSuggestions": 5,
    "uaida.completionDelay": 300
}
```

## 🔧 Troubleshooting

### Common Issues

#### 1. Server Won't Start
```bash
# Check if port is in use
lsof -i :8080

# Try different port
UAIDA_PORT=8081 make dev
```

#### 2. AI Model Not Loading
```bash
# Check model directory
ls -la models/

# Download model manually
curl -L https://huggingface.co/model-url -o models/model.bin
```

#### 3. VS Code Extension Not Working
```bash
# Reinstall extension
make extension-install

# Check VS Code developer console
# Help > Toggle Developer Tools
```

#### 4. Permission Issues
```bash
# Fix permissions
chmod +x scripts/*
sudo chown -R $USER:$USER .
```

### Getting Help

- **GitHub Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Discord**: Real-time community support
- **Documentation**: Comprehensive guides and API docs

## 🎓 Next Steps

### Learn More
- [API Documentation](API.md)
- [Architecture Overview](ARCHITECTURE.md)
- [Contributing Guide](../CONTRIBUTING.md)
- [Example Projects](../examples/)

### Advanced Usage
- [Custom Model Training](CUSTOM_MODELS.md)
- [Enterprise Deployment](ENTERPRISE.md)
- [Plugin Development](PLUGIN_DEVELOPMENT.md)
- [Performance Tuning](PERFORMANCE.md)

### Community
- [GitHub Discussions](https://github.com/yourusername/universal-ai-dev-assistant/discussions)
- [Discord Server](https://discord.gg/uaida)
- [Twitter](https://twitter.com/uaida_dev)
- [Blog](https://blog.uaida.dev)

## 🎉 You're Ready!

Congratulations! You now have a powerful AI development assistant at your fingertips. 

**Happy coding!** 🚀
# 🚀 UAIDA VS Code Extension

Universal AI Development Assistant extension for Visual Studio Code - Multi-provider AI assistance with cost optimization and Turkish support.

## ✨ Features

### 🤖 **Multi-Provider AI Completion**
- **OpenAI GPT-4/3.5** - Industry standard
- **OpenRouter** - Access to multiple models
- **Ollama** - Local, free models
- **Anthropic Claude** - Advanced reasoning
- **Automatic provider selection** based on cost and performance

### 💰 **Cost Optimization**
- Real-time cost tracking
- Provider cost comparison
- Daily/monthly usage reports
- Smart provider switching
- Budget alerts and warnings

### 🔍 **Code Analysis**
- **Security scanning** - Vulnerability detection
- **Performance analysis** - Optimization suggestions
- **Code quality** - Best practices
- **Bug detection** - Potential issues
- **Documentation** - Auto-generated docs

### 💬 **AI Chat Assistant**
- Context-aware conversations
- Code explanation and help
- Multi-language support
- Integration with current editor

### 🇹🇷 **Turkish Language Support**
- Native Turkish interface
- Turkish code comments
- Local developer community focus

## 🚀 Quick Start

### Installation

1. **Install from VS Code Marketplace** (coming soon)
   ```
   ext install uaida.uaida-vscode
   ```

2. **Or install from VSIX**
   ```bash
   code --install-extension uaida-vscode-0.1.0.vsix
   ```

### Setup

1. **Start UAIDA Backend**
   ```bash
   cd universal-ai-dev-assistant/backend
   cargo run
   ```

2. **Configure Extension**
   - Open VS Code Settings (`Ctrl+,`)
   - Search for "UAIDA"
   - Set your API URL (default: `http://localhost:8080`)
   - Choose your preferred AI provider

3. **Start Coding!**
   - Use `Ctrl+Shift+Space` for AI completion
   - Use `Ctrl+Shift+A` to open AI chat
   - Use `Ctrl+Shift+R` for code analysis

## 🎯 Usage

### Code Completion

**Automatic Completion:**
- Type code and get intelligent suggestions
- Multi-provider selection for best results
- Cost-optimized provider routing

**Manual Completion:**
- Select code and press `Ctrl+Shift+Space`
- Get context-aware completions
- Choose from multiple suggestions

### Code Analysis

**Security Analysis:**
```typescript
// Select code and press Ctrl+Shift+R
function login(username, password) {
    // UAIDA will detect potential security issues
    return database.query("SELECT * FROM users WHERE username = '" + username + "'");
}
```

**Performance Analysis:**
```python
# UAIDA will suggest optimizations
def slow_function():
    result = []
    for i in range(1000000):
        result.append(i * 2)
    return result
```

### AI Chat

**Open Chat Panel:**
- Press `Ctrl+Shift+A` or click UAIDA icon
- Ask questions about your code
- Get explanations and suggestions
- Insert generated code directly

**Example Conversations:**
```
You: "Explain this function"
UAIDA: "This function implements a binary search algorithm..."

You: "How can I optimize this code?"
UAIDA: "Here are 3 optimization suggestions..."

You: "Write a test for this function"
UAIDA: "Here's a comprehensive test suite..."
```

### Cost Tracking

**View Cost Dashboard:**
- Click on "Cost Dashboard" in sidebar
- See real-time usage statistics
- Compare provider costs
- Set budget alerts

**Cost Optimization:**
- Automatic provider switching
- Free Ollama for simple tasks
- Premium providers for complex work
- Daily/monthly budget tracking

## ⚙️ Configuration

### Basic Settings

```json
{
  "uaida.apiUrl": "http://localhost:8080",
  "uaida.defaultProvider": "auto",
  "uaida.autoComplete": true,
  "uaida.showCosts": true,
  "uaida.language": "en",
  "uaida.maxTokens": 1000,
  "uaida.temperature": 0.7
}
```

### Provider Settings

```json
{
  "uaida.providers": {
    "openai": {
      "apiKey": "your-openai-key",
      "model": "gpt-4"
    },
    "ollama": {
      "endpoint": "http://localhost:11434",
      "model": "codellama"
    }
  }
}
```

### Advanced Settings

```json
{
  "uaida.analysis": {
    "enableOnSave": true,
    "types": ["security", "performance", "quality"],
    "autoFix": false
  },
  "uaida.chat": {
    "contextLines": 20,
    "maxHistory": 100,
    "autoSave": true
  },
  "uaida.costs": {
    "dailyBudget": 5.0,
    "monthlyBudget": 100.0,
    "alertThreshold": 0.8
  }
}
```

## 🎨 Commands

| Command | Shortcut | Description |
|---------|----------|-------------|
| `UAIDA: Complete Code` | `Ctrl+Shift+Space` | AI code completion |
| `UAIDA: Analyze Code` | `Ctrl+Shift+R` | Code analysis |
| `UAIDA: Open AI Chat` | `Ctrl+Shift+A` | AI chat assistant |
| `UAIDA: Switch Provider` | - | Change AI provider |
| `UAIDA: Show Costs` | - | Cost dashboard |
| `UAIDA: Settings` | - | Open settings |

## 🔧 Troubleshooting

### Common Issues

**Backend Connection Failed:**
```
Error: Cannot connect to UAIDA backend
```
**Solution:** Make sure backend is running on `http://localhost:8080`

**High Costs Warning:**
```
Warning: High usage detected today ($5.00)
```
**Solution:** Switch to Ollama (free) or adjust usage patterns

**No Completions:**
```
No AI completions available
```
**Solution:** Check provider configuration and API keys

### Debug Mode

Enable debug logging:
```json
{
  "uaida.debug": true,
  "uaida.logLevel": "verbose"
}
```

Check VS Code Developer Console:
- `Help > Toggle Developer Tools`
- Look for UAIDA logs

## 🤝 Contributing

We welcome contributions! Here's how to get started:

### Development Setup

1. **Clone Repository**
   ```bash
   git clone https://github.com/Tehlikeli107/universal-ai-dev-assistant.git
   cd universal-ai-dev-assistant/extensions/vscode
   ```

2. **Install Dependencies**
   ```bash
   npm install
   ```

3. **Build Extension**
   ```bash
   npm run compile
   ```

4. **Test Extension**
   - Press `F5` to open Extension Development Host
   - Test your changes

### Building VSIX

```bash
npm install -g vsce
vsce package
```

## 📊 Roadmap

### v0.2.0 (Next Release)
- [ ] Standalone IDE preview
- [ ] Advanced security scanning
- [ ] Team collaboration features
- [ ] Turkish language interface

### v0.3.0
- [ ] Plugin marketplace
- [ ] Custom model training
- [ ] Enterprise features
- [ ] Mobile companion app

### v1.0.0
- [ ] Full feature parity with Cursor
- [ ] Advanced AI capabilities
- [ ] Enterprise deployment
- [ ] Multi-language support

## 📄 License

MIT License - see [LICENSE](../../LICENSE) for details.

## 🙏 Acknowledgments

- **VS Code Team** - Amazing platform
- **AI Providers** - OpenAI, Anthropic, etc.
- **Rust Community** - Backend technology
- **Turkish Developers** - Local community support

---

**Made with ❤️ by the UAIDA Team**

🌐 **Website:** https://uaida.dev  
📧 **Email:** support@uaida.dev  
💬 **Discord:** https://discord.gg/uaida  
🐙 **GitHub:** https://github.com/Tehlikeli107/universal-ai-dev-assistant
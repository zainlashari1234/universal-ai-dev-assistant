# 🔌 Universal AI Development Assistant - VS Code Extension

## 🚀 **Özellikleri**

### **AI-Powered Code Completion**
- **8 AI Provider** desteği (OpenRouter, OpenAI, Anthropic, Google, Groq, Together, Cohere, Ollama)
- **Gerçek zamanlı kod tamamlama** (Ctrl+Shift+Space)
- **Context-aware suggestions** - mevcut kodu anlayarak önerir
- **Multi-language support** - 20+ programlama dili

### **Advanced Code Analysis**
- **Security vulnerability detection** - güvenlik açıklarını tespit eder
- **Performance optimization** - performans önerileri
- **Code quality scoring** - kod kalitesi puanlama
- **Best practices** - en iyi uygulama önerileri

### **AI Chat Interface**
- **Code-specific chat** - kodunuz hakkında soru sorun
- **Debugging assistance** - hata ayıklama yardımı
- **Architecture advice** - mimari önerileri
- **Learning support** - öğrenme desteği

## 📦 **Kurulum**

### **1. VS Code Extension Kurulumu:**
```bash
# Extension klasörüne git
cd extensions/vscode

# Dependencies yükle
npm install

# TypeScript compile et
npm run compile

# Extension'ı package et
vsce package

# VS Code'a yükle
code --install-extension universal-ai-dev-assistant-1.0.0.vsix
```

### **2. Manuel Kurulum:**
```bash
# VS Code extensions klasörüne kopyala
cp -r extensions/vscode ~/.vscode/extensions/universal-ai-dev-assistant

# VS Code'u yeniden başlat
```

### **3. Development Mode:**
```bash
# VS Code'da F5 tuşuna bas
# Veya Command Palette'te "Developer: Reload Window"
```

## ⚙️ **Konfigürasyon**

### **VS Code Settings (settings.json):**
```json
{
  "uaida.apiUrl": "http://localhost:8080",
  "uaida.apiKey": "your-api-key-here",
  "uaida.defaultProvider": "openai",
  "uaida.maxTokens": 100,
  "uaida.temperature": 0.7
}
```

### **Environment Variables:**
```bash
export UAIDA_API_URL="http://localhost:8080"
export UAIDA_API_KEY="your-api-key"
export UAIDA_DEFAULT_PROVIDER="openai"
```

## 🎮 **Kullanım**

### **Keyboard Shortcuts:**
- **Ctrl+Shift+Space**: AI Code Completion
- **Ctrl+Shift+C**: AI Chat
- **Ctrl+Shift+A**: Code Analysis

### **Command Palette:**
- `UAIDA: AI Complete Code` - Kod tamamlama
- `UAIDA: Analyze Code` - Kod analizi
- `UAIDA: Open AI Chat` - AI chat aç
- `UAIDA: Explain Code` - Kodu açıkla
- `UAIDA: Refactor Code` - Kod refactor et
- `UAIDA: Generate Tests` - Test oluştur

### **Context Menu:**
- Sağ tık → `AI Complete Code`
- Sağ tık → `Analyze Code`
- Sağ tık → `Explain Code`

## 🔧 **Geliştirme**

### **Extension Geliştirme:**
```bash
# Development environment
cd extensions/vscode
npm install
npm run watch

# VS Code'da F5 - Extension Development Host açılır
```

### **Debug Mode:**
```bash
# VS Code'da Debug Console'u aç
# Extension loglarını gör
console.log('UAIDA Extension Debug');
```

### **Testing:**
```bash
# Unit tests
npm test

# Integration tests
npm run test:integration
```

## 📊 **Features Roadmap**

### **Mevcut Özellikler (v1.0.0):**
- ✅ **Code Completion** - AI-powered suggestions
- ✅ **Code Analysis** - Security & performance
- ✅ **Multi-provider** - 8 AI providers
- ✅ **Configuration** - Customizable settings
- ✅ **Keyboard shortcuts** - Quick access

### **Gelecek Özellikler (v1.1.0+):**
- 🔄 **Real-time Chat** - Interactive AI conversations
- 🔄 **Code Explanation** - Natural language explanations
- 🔄 **Test Generation** - Automated test creation
- 🔄 **Refactoring** - AI-powered code improvements
- 🔄 **Documentation** - Auto-generated docs
- 🔄 **Voice Commands** - Voice-activated coding

## 🐛 **Troubleshooting**

### **Common Issues:**

#### **1. Extension Not Loading:**
```bash
# VS Code Developer Tools'u aç
Help → Toggle Developer Tools

# Console'da hataları kontrol et
# Extension'ı reload et
Developer: Reload Window
```

#### **2. API Connection Issues:**
```bash
# API server çalışıyor mu?
curl http://localhost:8080/health

# Network connectivity
ping localhost

# Firewall settings
sudo ufw status
```

#### **3. Authentication Problems:**
```bash
# API key doğru mu?
curl -H "Authorization: Bearer YOUR_API_KEY" http://localhost:8080/api/v1/providers

# Settings kontrol et
VS Code → Settings → Extensions → UAIDA
```

### **Debug Commands:**
```bash
# Extension logs
code --log-level debug

# Reset extension
rm -rf ~/.vscode/extensions/universal-ai-dev-assistant
```

## 📈 **Performance Tips**

### **Optimization:**
- **Cache responses** - Aynı prompt'ları cache'le
- **Batch requests** - Birden fazla isteği grupla
- **Timeout settings** - Uygun timeout değerleri
- **Provider selection** - Hızlı provider'ları tercih et

### **Best Practices:**
- **Meaningful prompts** - Açık ve net prompt'lar yaz
- **Context awareness** - Mevcut kodu dahil et
- **Language specific** - Dil-specific ayarlar kullan
- **Regular updates** - Extension'ı güncel tut

## 🤝 **Contributing**

### **Extension Geliştirmeye Katkı:**
```bash
# Fork the repository
git fork https://github.com/Tehlikeli107/universal-ai-dev-assistant

# Create feature branch
git checkout -b feature/vscode-improvement

# Make changes
# Test thoroughly
npm test

# Submit PR
git push origin feature/vscode-improvement
```

### **Bug Reports:**
- GitHub Issues kullan
- Detaylı açıklama yap
- Log dosyalarını ekle
- Reproduction steps ver

---

## 🎉 **Başarılı Kurulum Testi**

Extension'ın doğru çalıştığını test etmek için:

1. **VS Code'u aç**
2. **Bir kod dosyası oluştur** (örn: `test.py`)
3. **Kod yaz**: `def fibonacci(n):`
4. **Ctrl+Shift+Space** tuşlarına bas
5. **AI suggestion** gelirse ✅ **BAŞARILI!**

**🚀 Universal AI Development Assistant VS Code Extension kullanıma hazır!**
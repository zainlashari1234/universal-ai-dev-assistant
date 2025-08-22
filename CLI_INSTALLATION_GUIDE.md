# 🚀 Universal AI Dev Assistant - CLI Kurulum ve Kullanım Kılavuzu

## 📋 **CLI Kurulum Seçenekleri**

### **Seçenek 1: Docker ile Hızlı Kurulum (Önerilen)**

#### **1. Ön Gereksinimler:**
```bash
# Docker kurulu olmalı
docker --version
docker-compose --version

# Git kurulu olmalı
git --version
```

#### **2. Projeyi İndir:**
```bash
# Repository'yi clone et
git clone https://github.com/Tehlikeli107/universal-ai-dev-assistant.git
cd universal-ai-dev-assistant
```

#### **3. Hızlı Başlatma:**
```bash
# Tüm servisleri başlat
docker-compose up -d

# Servislerin durumunu kontrol et
docker-compose ps

# Logları izle
docker-compose logs -f
```

#### **4. CLI Kullanımı:**
```bash
# Health check
curl http://localhost:8080/health

# API test
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "def fibonacci(n):",
    "language": "python",
    "max_tokens": 100
  }'
```

### **Seçenek 2: Manuel Kurulum (Geliştirici)**

#### **1. Backend (Rust) Kurulumu:**
```bash
# Rust kurulumu
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Backend'i derle
cd backend
cargo build --release

# Veritabanını başlat
docker-compose up -d postgres redis

# Migration'ları çalıştır
cargo sqlx migrate run

# Backend'i çalıştır
cargo run --release
```

#### **2. Frontend (React) Kurulumu:**
```bash
# Node.js kurulumu (18+)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Frontend'i derle
cd frontend
npm install
npm run build

# Development server
npm run dev
```

### **Seçenek 3: Production Deployment**

#### **1. Kubernetes Deployment:**
```bash
# Kubernetes cluster'ında deploy et
cd k8s
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f postgres.yaml
kubectl apply -f backend.yaml
kubectl apply -f frontend.yaml
kubectl apply -f ingress.yaml
```

#### **2. Cloud Deployment:**
```bash
# AWS EKS
eksctl create cluster --name uaida-cluster

# Google GKE
gcloud container clusters create uaida-cluster

# Azure AKS
az aks create --name uaida-cluster
```

## 🔧 **CLI Komutları ve Kullanım**

### **Temel API Komutları:**

#### **1. Health Check:**
```bash
curl http://localhost:8080/health
```

#### **2. Kod Tamamlama:**
```bash
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "function calculateSum(a, b) {",
    "language": "javascript",
    "max_tokens": 50,
    "temperature": 0.7,
    "provider": "openai"
  }'
```

#### **3. Kod Analizi:**
```bash
curl -X POST http://localhost:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{
    "code": "def unsafe_function(user_input): exec(user_input)",
    "language": "python",
    "analysis_types": ["security", "performance"]
  }'
```

#### **4. Kod Arama:**
```bash
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "authentication function",
    "language": "rust",
    "limit": 10
  }'
```

#### **5. AI Sağlayıcıları Listele:**
```bash
curl http://localhost:8080/api/v1/providers
```

### **Gelişmiş CLI Kullanımı:**

#### **1. Batch İşlemler:**
```bash
# Birden fazla dosyayı analiz et
for file in *.py; do
  curl -X POST http://localhost:8080/api/v1/analyze \
    -H "Content-Type: application/json" \
    -d "{\"code\": \"$(cat $file)\", \"language\": \"python\"}"
done
```

#### **2. Streaming Responses:**
```bash
# Streaming kod tamamlama
curl -X POST http://localhost:8080/api/v1/complete/stream \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "class DatabaseManager:",
    "language": "python",
    "stream": true
  }'
```

#### **3. Authentication:**
```bash
# API key ile
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "code here", "language": "python"}'
```

## 🛠️ **CLI Araçları ve Scripts**

### **1. Hızlı Başlatma Script'i:**
```bash
# quick_start.sh oluştur
cat > quick_start.sh << 'EOF'
#!/bin/bash
echo "🚀 Universal AI Dev Assistant - Quick Start"

# Docker kontrol
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

# Servisleri başlat
echo "📦 Starting services..."
docker-compose up -d

# Health check
echo "🔍 Checking health..."
sleep 10
curl -s http://localhost:8080/health | jq '.' || echo "⚠️ Service starting..."

echo "✅ Universal AI Dev Assistant is ready!"
echo "🌐 Web UI: http://localhost:3000"
echo "📡 API: http://localhost:8080"
EOF

chmod +x quick_start.sh
./quick_start.sh
```

### **2. Test Script'i:**
```bash
# test_api.sh oluştur
cat > test_api.sh << 'EOF'
#!/bin/bash
echo "🧪 Testing Universal AI Dev Assistant API"

BASE_URL="http://localhost:8080"

# Test 1: Health
echo "1️⃣ Testing health endpoint..."
curl -s "$BASE_URL/health" | jq '.'

# Test 2: Completion
echo "2️⃣ Testing completion..."
curl -s -X POST "$BASE_URL/api/v1/complete" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "def hello():", "language": "python"}' | jq '.'

# Test 3: Providers
echo "3️⃣ Testing providers..."
curl -s "$BASE_URL/api/v1/providers" | jq '.'

echo "✅ API tests completed!"
EOF

chmod +x test_api.sh
./test_api.sh
```

### **3. Development Helper:**
```bash
# dev_helper.sh oluştur
cat > dev_helper.sh << 'EOF'
#!/bin/bash

case "$1" in
  "start")
    echo "🚀 Starting development environment..."
    docker-compose up -d postgres redis
    cd backend && cargo run &
    cd frontend && npm run dev &
    ;;
  "stop")
    echo "🛑 Stopping services..."
    docker-compose down
    pkill -f "cargo run"
    pkill -f "npm run dev"
    ;;
  "logs")
    echo "📋 Showing logs..."
    docker-compose logs -f
    ;;
  "test")
    echo "🧪 Running tests..."
    ./test_api.sh
    ;;
  *)
    echo "Usage: $0 {start|stop|logs|test}"
    ;;
esac
EOF

chmod +x dev_helper.sh
```

## 📊 **Monitoring ve Debugging**

### **1. Log Monitoring:**
```bash
# Tüm servislerin logları
docker-compose logs -f

# Sadece backend logları
docker-compose logs -f uaida-backend

# Sadece database logları
docker-compose logs -f postgres
```

### **2. Performance Monitoring:**
```bash
# Resource kullanımı
docker stats

# API response times
curl -w "@curl-format.txt" -s http://localhost:8080/health

# curl-format.txt dosyası:
cat > curl-format.txt << 'EOF'
     time_namelookup:  %{time_namelookup}\n
        time_connect:  %{time_connect}\n
     time_appconnect:  %{time_appconnect}\n
    time_pretransfer:  %{time_pretransfer}\n
       time_redirect:  %{time_redirect}\n
  time_starttransfer:  %{time_starttransfer}\n
                     ----------\n
          time_total:  %{time_total}\n
EOF
```

### **3. Troubleshooting:**
```bash
# Port kontrolü
netstat -tulpn | grep :8080

# Service durumu
docker-compose ps

# Container içine gir
docker-compose exec uaida-backend bash

# Database bağlantısı test et
docker-compose exec postgres psql -U uaida -d uaida -c "SELECT 1;"
```

## 🎯 **Kullanım Senaryoları**

### **1. Geliştirici Workflow:**
```bash
# 1. Projeyi başlat
./quick_start.sh

# 2. Kod yaz ve AI'dan yardım al
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Content-Type: application/json" \
  -d '{"prompt": "// TODO: implement user authentication", "language": "javascript"}'

# 3. Kodu analiz et
curl -X POST http://localhost:8080/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{"code": "$(cat myfile.js)", "language": "javascript"}'
```

### **2. CI/CD Integration:**
```bash
# GitHub Actions'da kullanım
- name: Code Analysis
  run: |
    curl -X POST ${{ secrets.UAIDA_URL }}/api/v1/analyze \
      -H "Authorization: Bearer ${{ secrets.UAIDA_API_KEY }}" \
      -H "Content-Type: application/json" \
      -d '{"code": "$(cat src/main.rs)", "language": "rust"}'
```

### **3. Team Usage:**
```bash
# Team için shared instance
docker-compose -f docker-compose.production.yml up -d

# Load balancer ile
nginx -c nginx.conf
```

---

## ✅ **Kurulum Doğrulama**

### **Başarılı Kurulum Kontrolleri:**
```bash
# 1. Servisler çalışıyor mu?
curl http://localhost:8080/health
# Beklenen: {"status": "healthy", "version": "6.4.0"}

# 2. AI providers mevcut mu?
curl http://localhost:8080/api/v1/providers
# Beklenen: 8 provider listesi

# 3. Kod tamamlama çalışıyor mu?
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Content-Type: application/json" \
  -d '{"prompt": "def test():", "language": "python"}'
# Beklenen: AI suggestions

# 4. Web UI erişilebilir mi?
curl http://localhost:3000
# Beklenen: React app HTML
```

**🎉 Tüm kontroller başarılıysa Universal AI Development Assistant kullanıma hazır!**
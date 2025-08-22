# 📚 Universal AI Development Assistant - Complete User Guide

## 🎯 **Hoş Geldiniz!**

Universal AI Development Assistant'a hoş geldiniz! Bu kapsamlı kılavuz, platformunuzu kullanmaya başlamanız için gereken her şeyi içerir.

---

## 🚀 **Hızlı Başlangıç**

### **1. Platform Özellikleri:**
```yaml
✅ 8 AI Sağlayıcısı:
  - OpenRouter, OpenAI, Anthropic, Google
  - Groq, Together, Cohere, Ollama

✅ Gelişmiş Özellikler:
  - Gerçek zamanlı kod tamamlama
  - Semantik kod arama
  - Güvenlik analizi
  - Performans izleme
  - Çoklu dil desteği

✅ Kurumsal Güvenlik:
  - JWT kimlik doğrulama
  - RBAC yetkilendirme
  - API key şifreleme
  - Audit logging
```

### **2. İlk Kurulum:**
```bash
# Hızlı başlangıç
cd universal-ai-dev-assistant
./QUICK_PRODUCTION_START.sh

# Manuel kurulum
docker compose up -d
curl http://localhost:8080/health
```

---

## 🔧 **Kullanım Kılavuzu**

### **API Kullanımı:**

#### **1. Kimlik Doğrulama:**
```bash
# API key ile
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:8080/api/v1/info

# JWT token ile
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:8080/api/v1/complete
```

#### **2. Kod Tamamlama:**
```bash
curl -X POST http://localhost:8080/api/v1/complete \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "def fibonacci(n):",
    "language": "python",
    "max_tokens": 100,
    "temperature": 0.7,
    "provider": "openai"
  }'
```

#### **3. Kod Arama:**
```bash
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "authentication function",
    "language": "rust",
    "file_path": "src/auth/",
    "limit": 10
  }'
```

#### **4. Güvenlik Analizi:**
```bash
curl -X POST http://localhost:8080/api/v1/analyze/security \
  -H "Content-Type: application/json" \
  -d '{
    "code": "your_code_here",
    "language": "javascript",
    "check_types": ["xss", "sql_injection", "auth"]
  }'
```

### **Frontend Kullanımı:**

#### **1. Web Arayüzü:**
```
🌐 Ana Sayfa: http://localhost:3000
📊 Dashboard: http://localhost:3000/dashboard
⚙️ Ayarlar: http://localhost:3000/settings
📈 Analytics: http://localhost:3000/analytics
```

#### **2. Temel Özellikler:**
- **Kod Editörü**: Gerçek zamanlı tamamlama
- **Proje Yönetimi**: Çoklu proje desteği
- **AI Sağlayıcı Seçimi**: 8 farklı AI modeli
- **Kullanım İstatistikleri**: Detaylı analytics

---

## 🛠️ **Yönetici Kılavuzu**

### **Sistem Yönetimi:**

#### **1. Servis Yönetimi:**
```bash
# Servisleri başlat
docker compose up -d

# Servisleri durdur
docker compose down

# Logları görüntüle
docker compose logs -f

# Servis durumunu kontrol et
docker compose ps
```

#### **2. Database Yönetimi:**
```bash
# Database backup
docker compose exec postgres pg_dump -U uaida uaida > backup.sql

# Database restore
docker compose exec -T postgres psql -U uaida uaida < backup.sql

# Migration çalıştır
cd backend && cargo sqlx migrate run
```

#### **3. Monitoring:**
```bash
# Prometheus metrics
curl http://localhost:9090/metrics

# Grafana dashboard
open http://localhost:3001

# Health checks
curl http://localhost:8080/health
curl http://localhost:8080/api/v1/database/health
```

### **Güvenlik Yönetimi:**

#### **1. API Key Yönetimi:**
```bash
# Yeni API key oluştur
curl -X POST http://localhost:8080/api/v1/auth/api-keys \
  -H "Authorization: Bearer ADMIN_TOKEN" \
  -d '{"name": "production-key", "permissions": ["read", "write"]}'

# API key listele
curl http://localhost:8080/api/v1/auth/api-keys \
  -H "Authorization: Bearer ADMIN_TOKEN"

# API key iptal et
curl -X DELETE http://localhost:8080/api/v1/auth/api-keys/KEY_ID \
  -H "Authorization: Bearer ADMIN_TOKEN"
```

#### **2. Kullanıcı Yönetimi:**
```bash
# Kullanıcı oluştur
curl -X POST http://localhost:8080/api/v1/auth/users \
  -d '{"username": "newuser", "email": "user@example.com", "role": "developer"}'

# Kullanıcı rolü güncelle
curl -X PUT http://localhost:8080/api/v1/auth/users/USER_ID/role \
  -d '{"role": "admin"}'
```

---

## 📊 **API Referansı**

### **Temel Endpoint'ler:**

#### **Health & Info:**
```
GET  /health                    - Sistem sağlığı
GET  /api/v1/info              - API bilgileri
GET  /api/v1/providers         - AI sağlayıcı listesi
GET  /metrics                  - Prometheus metrics
```

#### **Authentication:**
```
POST /api/v1/auth/login        - Kullanıcı girişi
POST /api/v1/auth/register     - Kullanıcı kaydı
POST /api/v1/auth/refresh      - Token yenileme
POST /api/v1/auth/logout       - Çıkış
```

#### **Code Operations:**
```
POST /api/v1/complete          - Kod tamamlama
POST /api/v1/analyze           - Kod analizi
POST /api/v1/search            - Kod arama
POST /api/v1/explain           - Kod açıklama
```

#### **Project Management:**
```
GET    /api/v1/projects        - Proje listesi
POST   /api/v1/projects        - Yeni proje
GET    /api/v1/projects/:id    - Proje detayı
PUT    /api/v1/projects/:id    - Proje güncelle
DELETE /api/v1/projects/:id    - Proje sil
```

### **Response Formatları:**

#### **Başarılı Response:**
```json
{
  "success": true,
  "data": {
    "completion": "generated code here",
    "provider": "openai",
    "model": "gpt-4",
    "tokens_used": 150,
    "processing_time_ms": 245
  },
  "metadata": {
    "request_id": "uuid",
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

#### **Hata Response:**
```json
{
  "success": false,
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Missing required parameter: prompt",
    "details": {
      "field": "prompt",
      "expected": "string",
      "received": "null"
    }
  },
  "request_id": "uuid"
}
```

---

## 🔧 **Sorun Giderme**

### **Yaygın Sorunlar:**

#### **1. Backend Bağlantı Sorunu:**
```bash
# Servis durumunu kontrol et
docker compose ps

# Backend loglarını kontrol et
docker compose logs uaida-backend

# Health check
curl http://localhost:8080/health
```

#### **2. Database Bağlantı Sorunu:**
```bash
# PostgreSQL durumu
docker compose exec postgres pg_isready -U uaida

# Database bağlantısı test et
docker compose exec postgres psql -U uaida -d uaida -c "SELECT 1;"
```

#### **3. Performance Sorunları:**
```bash
# Resource kullanımı
docker stats

# Slow query log
docker compose logs postgres | grep "slow"

# Cache hit rate
docker compose exec redis redis-cli info stats
```

### **Debug Modları:**

#### **1. Verbose Logging:**
```bash
# Environment variable
export RUST_LOG=debug

# Docker compose
RUST_LOG=debug docker compose up
```

#### **2. Database Debug:**
```bash
# SQL query logging
export SQLX_LOGGING=true
```

---

## 🎯 **En İyi Uygulamalar**

### **Performance:**
```yaml
✅ Connection pooling kullanın
✅ Cache stratejisi uygulayın
✅ Rate limiting ayarlayın
✅ Monitoring kurun
✅ Regular backup alın
```

### **Security:**
```yaml
✅ HTTPS kullanın (production)
✅ API key'leri güvenli saklayın
✅ Regular security audit yapın
✅ Input validation uygulayın
✅ Audit logging aktif tutun
```

### **Scalability:**
```yaml
✅ Horizontal scaling planlayın
✅ Database read replica kullanın
✅ CDN entegrasyonu yapın
✅ Load balancer kurun
✅ Auto-scaling ayarlayın
```

---

## 🎉 **Tebrikler!**

Universal AI Development Assistant'ı başarıyla kurduğunuz ve yapılandırdığınız için tebrikler! 

### **Artık şunları yapabilirsiniz:**
✅ **8 farklı AI sağlayıcısı** ile kod geliştirme
✅ **Gerçek zamanlı kod tamamlama** kullanma
✅ **Gelişmiş kod arama** ve analiz
✅ **Kurumsal güvenlik** özellikleri
✅ **Performance monitoring** ve analytics
✅ **Scalable deployment** seçenekleri

### **Destek ve Topluluk:**
- 📚 **Dokümantasyon**: Kapsamlı kılavuzlar
- 🐛 **Issue Tracking**: GitHub issues
- 💬 **Community**: Discord/Slack kanalları
- 📧 **Support**: Email desteği
- 🎓 **Training**: Video tutorials

**Platformunuz artık GitHub Copilot, Cursor AI ve diğer sektör liderleriyle yarışmaya hazır!** 🚀
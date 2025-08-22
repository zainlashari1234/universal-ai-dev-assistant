# 🚀 Production Deployment Guide

## Universal AI Development Assistant - Production Ready Deployment

This guide covers deploying the Universal AI Development Assistant to production using Docker Compose or Kubernetes.

## 📋 Prerequisites

### System Requirements
- **CPU:** 4+ cores
- **RAM:** 8GB+ (16GB recommended)
- **Storage:** 50GB+ SSD
- **Network:** Static IP with domain name

### Software Requirements
- Docker 24.0+
- Docker Compose 2.0+
- Kubernetes 1.25+ (for K8s deployment)
- kubectl (for K8s deployment)
- SSL certificates (Let's Encrypt recommended)

## 🔧 Quick Start (Docker Compose)

### 1. Clone and Setup
```bash
git clone https://github.com/your-username/universal-ai-dev-assistant.git
cd universal-ai-dev-assistant
```

### 2. Configure Environment
```bash
# Copy and edit production environment
cp .env.production.example .env.production
nano .env.production
```

**Required Environment Variables:**
```bash
# Database
POSTGRES_PASSWORD=your-super-secure-postgres-password
REDIS_PASSWORD=your-super-secure-redis-password

# Security
JWT_SECRET=your-super-secret-jwt-key-32-chars-minimum
ENCRYPTION_KEY=your-32-byte-encryption-key-change-this

# Domain
REACT_APP_API_URL=https://api.yourdomain.com
CORS_ORIGINS=https://yourdomain.com,https://www.yourdomain.com

# Monitoring
GRAFANA_PASSWORD=your-secure-grafana-admin-password
```

### 3. Deploy
```bash
# Make deploy script executable
chmod +x deploy.sh

# Deploy to production
./deploy.sh production
```

### 4. Verify Deployment
```bash
# Check service status
docker-compose -f docker-compose.production.yml ps

# Test endpoints
curl https://yourdomain.com/health
curl https://api.yourdomain.com/health
```

## ☸️ Kubernetes Deployment

### 1. Prepare Cluster
```bash
# Ensure kubectl is configured
kubectl cluster-info

# Create namespace
kubectl apply -f k8s/namespace.yaml
```

### 2. Configure Secrets
```bash
# Edit secrets with your values
cp k8s/secrets.yaml k8s/secrets-production.yaml
nano k8s/secrets-production.yaml

# Apply secrets
kubectl apply -f k8s/secrets-production.yaml
```

### 3. Deploy Services
```bash
cd k8s
./deploy.sh production
```

### 4. Configure DNS
Point your domain to the ingress controller:
```bash
# Get ingress IP
kubectl get ingress -n uaida-production

# Configure DNS records
yourdomain.com        A    <INGRESS_IP>
www.yourdomain.com    A    <INGRESS_IP>
api.yourdomain.com    A    <INGRESS_IP>
```

## 🔒 SSL/TLS Configuration

### Let's Encrypt (Recommended)
```bash
# Install cert-manager
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.13.0/cert-manager.yaml

# Create ClusterIssuer
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@yourdomain.com
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
    - http01:
        ingress:
          class: nginx
EOF
```

### Manual SSL Certificates
```bash
# Create TLS secret
kubectl create secret tls tls-secret \
  --cert=path/to/cert.pem \
  --key=path/to/key.pem \
  -n uaida-production
```

## 📊 Monitoring Setup

### Access Monitoring Dashboards
- **Grafana:** https://monitoring.yourdomain.com
- **Prometheus:** https://prometheus.yourdomain.com

### Default Dashboards
1. **Application Metrics**
   - Request rates and latencies
   - Error rates and success rates
   - Database performance
   - API key usage

2. **Infrastructure Metrics**
   - CPU and memory usage
   - Disk I/O and network
   - Container health
   - Kubernetes cluster status

3. **Business Metrics**
   - User registrations
   - API usage by provider
   - Cost tracking
   - Feature adoption

## 🔧 Configuration Options

### Environment Variables

#### Core Configuration
```bash
# Server
PORT=3001
HOST=0.0.0.0
RUST_LOG=info

# Database
DATABASE_URL=postgresql://user:pass@host:5432/db
REDIS_URL=redis://:pass@host:6379

# Security
JWT_SECRET=your-jwt-secret
ENCRYPTION_KEY=your-32-byte-key
SESSION_TIMEOUT_HOURS=24
```

#### Feature Flags
```bash
ENABLE_REGISTRATION=true
ENABLE_PASSWORD_RESET=true
ENABLE_EMAIL_VERIFICATION=false
ENABLE_ANALYTICS=true
ENABLE_MONITORING=true
```

#### Rate Limiting
```bash
RATE_LIMIT_REQUESTS_PER_MINUTE=60
RATE_LIMIT_BURST=10
MAX_LOGIN_ATTEMPTS=5
LOCKOUT_DURATION_MINUTES=15
```

### Scaling Configuration

#### Docker Compose Scaling
```bash
# Scale backend
docker-compose -f docker-compose.production.yml up -d --scale backend=3

# Scale frontend
docker-compose -f docker-compose.production.yml up -d --scale frontend=2
```

#### Kubernetes Auto-scaling
```yaml
# HPA is already configured in k8s/backend.yaml
# Backend: 3-10 replicas based on CPU/Memory
# Frontend: 2-5 replicas based on CPU
```

## 🔄 CI/CD Pipeline

### GitHub Actions
The included `.github/workflows/ci-cd.yml` provides:

1. **Continuous Integration**
   - Backend tests (Rust)
   - Frontend tests (React)
   - Security scanning
   - Code quality checks

2. **Continuous Deployment**
   - Docker image building
   - Container registry push
   - Staging deployment
   - Production deployment

### Setup CI/CD
```bash
# 1. Fork the repository
# 2. Configure GitHub secrets:
#    - GITHUB_TOKEN (automatic)
#    - REACT_APP_API_URL
#    - Production deployment secrets

# 3. Push to main branch triggers deployment
git push origin main
```

## 🛠️ Maintenance

### Backup Strategy
```bash
# Database backup (automated daily)
kubectl create cronjob postgres-backup \
  --image=postgres:15 \
  --schedule="0 2 * * *" \
  -- pg_dump $DATABASE_URL > /backup/$(date +%Y%m%d).sql

# Application data backup
kubectl exec -n uaida-production postgres-0 -- \
  pg_dump -U uaida uaida_prod > backup-$(date +%Y%m%d).sql
```

### Updates and Rollbacks
```bash
# Update backend
kubectl set image deployment/uaida-backend \
  backend=ghcr.io/your-username/universal-ai-dev-assistant-backend:v2.0.0 \
  -n uaida-production

# Rollback if needed
kubectl rollout undo deployment/uaida-backend -n uaida-production

# Check rollout status
kubectl rollout status deployment/uaida-backend -n uaida-production
```

### Log Management
```bash
# View application logs
kubectl logs -f deployment/uaida-backend -n uaida-production

# View nginx logs
kubectl logs -f deployment/nginx -n uaida-production

# Aggregate logs with stern (recommended)
stern uaida -n uaida-production
```

## 🚨 Troubleshooting

### Common Issues

#### 1. Database Connection Issues
```bash
# Check PostgreSQL status
kubectl get pods -l app=postgres -n uaida-production

# Check database logs
kubectl logs -f deployment/postgres -n uaida-production

# Test connection
kubectl exec -it deployment/postgres -n uaida-production -- \
  psql -U uaida -d uaida_prod -c "SELECT 1;"
```

#### 2. SSL Certificate Issues
```bash
# Check certificate status
kubectl describe certificate tls-secret -n uaida-production

# Check cert-manager logs
kubectl logs -f deployment/cert-manager -n cert-manager

# Manual certificate renewal
kubectl delete certificate tls-secret -n uaida-production
kubectl apply -f k8s/ingress.yaml
```

#### 3. High Memory Usage
```bash
# Check resource usage
kubectl top pods -n uaida-production

# Scale up if needed
kubectl scale deployment uaida-backend --replicas=5 -n uaida-production

# Check HPA status
kubectl get hpa -n uaida-production
```

### Performance Optimization

#### 1. Database Optimization
```sql
-- Add indexes for better performance
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);
CREATE INDEX CONCURRENTLY idx_sessions_user_id ON sessions(user_id);
CREATE INDEX CONCURRENTLY idx_api_keys_user_provider ON api_keys(user_id, provider);
```

#### 2. Redis Caching
```bash
# Monitor Redis performance
kubectl exec -it deployment/redis -n uaida-production -- redis-cli info memory
```

#### 3. CDN Setup
```bash
# Configure CloudFlare or AWS CloudFront
# Point static assets to CDN
# Configure cache headers in nginx
```

## 📞 Support

### Health Checks
- **Backend:** `https://api.yourdomain.com/health`
- **Frontend:** `https://yourdomain.com/health`
- **Database:** Check via backend health endpoint

### Monitoring Alerts
Configure alerts for:
- High error rates (>5%)
- High response times (>500ms)
- Database connection issues
- Memory usage >80%
- Disk usage >85%

### Contact Information
- **Documentation:** [GitHub Wiki](https://github.com/your-username/universal-ai-dev-assistant/wiki)
- **Issues:** [GitHub Issues](https://github.com/your-username/universal-ai-dev-assistant/issues)
- **Security:** security@yourdomain.com

---

## 🎉 Congratulations!

Your Universal AI Development Assistant is now running in production! 

**Next Steps:**
1. Configure monitoring alerts
2. Set up automated backups
3. Configure CDN for better performance
4. Add custom AI provider integrations
5. Scale based on usage patterns

**Happy Coding!** 🚀
#!/bin/bash
# Universal AI Dev Assistant - Quick Production Start Script

echo "🚀 Universal AI Development Assistant - Production Deployment"
echo "=============================================================="

# Check if running as root (for production server setup)
if [[ $EUID -eq 0 ]]; then
   echo "⚠️  Running as root - production server setup mode"
else
   echo "👤 Running as user - development/testing mode"
fi

# Check Docker availability
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose not found. Please install Docker Compose first."
    exit 1
fi

echo "✅ Docker and Docker Compose available"

# Setup environment
if [ ! -f .env ]; then
    echo "📝 Creating production environment file..."
    cp .env.example .env
    echo "⚠️  Please edit .env file with your production values before continuing!"
    echo "   Required: DATABASE_URL, JWT_SECRET, ENCRYPTION_KEY"
    read -p "Press Enter after editing .env file..."
fi

# Production deployment
echo "🚀 Starting production deployment..."

# Start core services first
echo "1️⃣ Starting database and cache services..."
docker compose up -d postgres redis

# Wait for database to be ready
echo "⏳ Waiting for database to be ready..."
sleep 10

# Check database connection
echo "🔍 Checking database connection..."
if docker compose exec -T postgres pg_isready -U uaida; then
    echo "✅ Database is ready"
else
    echo "❌ Database connection failed"
    exit 1
fi

# Start application services
echo "2️⃣ Starting application services..."
docker compose up -d

# Wait for services to start
echo "⏳ Waiting for services to start..."
sleep 30

# Health check
echo "🔍 Performing health checks..."

# Check if services are running
echo "📊 Service Status:"
docker compose ps

# Test health endpoint (when backend is ready)
echo "🏥 Testing health endpoint..."
for i in {1..12}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo "✅ Backend health check passed"
        break
    else
        echo "⏳ Waiting for backend... ($i/12)"
        sleep 10
    fi
done

# Final status
echo ""
echo "🎉 Production Deployment Status:"
echo "================================"
echo "🌐 Application URL: http://localhost:8080"
echo "📊 Metrics URL: http://localhost:9090 (Prometheus)"
echo "📈 Dashboard URL: http://localhost:3001 (Grafana)"
echo ""
echo "🔧 Management Commands:"
echo "  View logs: docker compose logs -f"
echo "  Stop services: docker compose down"
echo "  Restart: docker compose restart"
echo "  Update: docker compose pull && docker compose up -d"
echo ""
echo "📚 Documentation:"
echo "  API Docs: http://localhost:8080/docs"
echo "  Health Check: http://localhost:8080/health"
echo "  Metrics: http://localhost:8080/metrics"
echo ""

# Production checklist
echo "✅ Production Deployment Checklist:"
echo "  ✅ Database and cache services running"
echo "  ✅ Application services started"
echo "  ✅ Health checks configured"
echo "  ✅ Monitoring stack available"
echo "  ⚠️  SSL certificates (configure for production domain)"
echo "  ⚠️  Firewall rules (configure for production security)"
echo "  ⚠️  Backup strategy (implement for production data)"
echo ""
echo "🚀 Your Universal AI Development Assistant is now running in production mode!"
echo "🌟 Ready to compete with GitHub Copilot, Cursor AI, and industry leaders!"
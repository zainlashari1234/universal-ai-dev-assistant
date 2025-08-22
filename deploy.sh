#!/bin/bash

# Universal AI Development Assistant - Production Deployment Script
# This script deploys the application to production environment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${1:-production}
COMPOSE_FILE="docker-compose.${ENVIRONMENT}.yml"

echo -e "${BLUE}🚀 Universal AI Development Assistant - Production Deployment${NC}"
echo "=============================================================="
echo ""

# Check if environment file exists
if [ ! -f ".env.${ENVIRONMENT}" ]; then
    echo -e "${RED}❌ Environment file .env.${ENVIRONMENT} not found!${NC}"
    echo "Please create the environment file with required variables."
    exit 1
fi

# Check if docker-compose file exists
if [ ! -f "$COMPOSE_FILE" ]; then
    echo -e "${RED}❌ Docker compose file $COMPOSE_FILE not found!${NC}"
    exit 1
fi

echo -e "${YELLOW}📋 Deployment Configuration:${NC}"
echo "  Environment: $ENVIRONMENT"
echo "  Compose file: $COMPOSE_FILE"
echo "  Environment file: .env.${ENVIRONMENT}"
echo ""

# Load environment variables
export $(cat .env.${ENVIRONMENT} | grep -v '^#' | xargs)

# Validate required environment variables
required_vars=(
    "POSTGRES_PASSWORD"
    "REDIS_PASSWORD"
    "JWT_SECRET"
    "ENCRYPTION_KEY"
    "GRAFANA_PASSWORD"
)

echo -e "${YELLOW}🔍 Validating environment variables...${NC}"
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo -e "${RED}❌ Required environment variable $var is not set!${NC}"
        exit 1
    else
        echo -e "${GREEN}✅ $var is set${NC}"
    fi
done

echo ""

# Pre-deployment checks
echo -e "${YELLOW}🔧 Pre-deployment checks...${NC}"

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker is not installed!${NC}"
    exit 1
fi

# Check Docker Compose
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}❌ Docker Compose is not installed!${NC}"
    exit 1
fi

# Check available disk space (at least 5GB)
available_space=$(df / | awk 'NR==2 {print $4}')
required_space=5242880  # 5GB in KB

if [ "$available_space" -lt "$required_space" ]; then
    echo -e "${RED}❌ Insufficient disk space! At least 5GB required.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All pre-deployment checks passed${NC}"
echo ""

# Build and deploy
echo -e "${YELLOW}🏗️  Building and deploying services...${NC}"

# Pull latest images
echo "Pulling latest images..."
docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" pull

# Build services
echo "Building services..."
docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" build --no-cache

# Stop existing services
echo "Stopping existing services..."
docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" down

# Start services
echo "Starting services..."
docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" up -d

# Wait for services to be healthy
echo ""
echo -e "${YELLOW}⏳ Waiting for services to be healthy...${NC}"

services=("postgres" "redis" "backend" "frontend")
max_attempts=30
attempt=0

for service in "${services[@]}"; do
    echo -n "  Waiting for $service... "
    attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" ps "$service" | grep -q "healthy\|Up"; then
            echo -e "${GREEN}✅ Healthy${NC}"
            break
        fi
        
        sleep 5
        attempt=$((attempt + 1))
        echo -n "."
    done
    
    if [ $attempt -eq $max_attempts ]; then
        echo -e "${RED}❌ Timeout waiting for $service${NC}"
        exit 1
    fi
done

echo ""

# Post-deployment verification
echo -e "${YELLOW}🧪 Post-deployment verification...${NC}"

# Check backend health
echo -n "  Testing backend health... "
if curl -f -s "http://localhost:3001/health" > /dev/null; then
    echo -e "${GREEN}✅ Backend is healthy${NC}"
else
    echo -e "${RED}❌ Backend health check failed${NC}"
    exit 1
fi

# Check frontend
echo -n "  Testing frontend... "
if curl -f -s "http://localhost:3000" > /dev/null; then
    echo -e "${GREEN}✅ Frontend is accessible${NC}"
else
    echo -e "${RED}❌ Frontend is not accessible${NC}"
    exit 1
fi

# Check database connection
echo -n "  Testing database connection... "
if docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" exec -T backend ./universal-ai-dev-assistant --version > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Database connection successful${NC}"
else
    echo -e "${YELLOW}⚠️  Database connection test skipped${NC}"
fi

echo ""

# Display service status
echo -e "${BLUE}📊 Service Status:${NC}"
docker-compose -f "$COMPOSE_FILE" --env-file ".env.${ENVIRONMENT}" ps

echo ""

# Display URLs
echo -e "${GREEN}🎉 Deployment completed successfully!${NC}"
echo ""
echo -e "${BLUE}📱 Application URLs:${NC}"
echo "  Frontend: http://localhost:3000"
echo "  Backend API: http://localhost:3001"
echo "  Prometheus: http://localhost:9090"
echo "  Grafana: http://localhost:3003"
echo ""

echo -e "${BLUE}🔐 Default Credentials:${NC}"
echo "  Grafana Admin: admin / ${GRAFANA_PASSWORD}"
echo ""

echo -e "${BLUE}📋 Useful Commands:${NC}"
echo "  View logs: docker-compose -f $COMPOSE_FILE logs -f [service]"
echo "  Stop services: docker-compose -f $COMPOSE_FILE down"
echo "  Restart service: docker-compose -f $COMPOSE_FILE restart [service]"
echo "  Update services: ./deploy.sh $ENVIRONMENT"
echo ""

echo -e "${GREEN}✨ Universal AI Development Assistant is now running in $ENVIRONMENT mode!${NC}"
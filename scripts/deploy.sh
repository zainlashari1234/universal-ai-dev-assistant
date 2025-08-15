#!/bin/bash

# UAIDA Production Deployment Script
set -e

echo "🚀 Starting UAIDA Production Deployment..."

# Configuration
ENVIRONMENT=${1:-production}
COMPOSE_FILE="docker-compose.yml"
BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Pre-deployment checks
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    # Check if ports are available
    if lsof -Pi :8080 -sTCP:LISTEN -t >/dev/null; then
        log_warning "Port 8080 is already in use"
    fi
    
    if lsof -Pi :3000 -sTCP:LISTEN -t >/dev/null; then
        log_warning "Port 3000 (Grafana) is already in use"
    fi
    
    log_success "Prerequisites check completed"
}

# Create backup of existing deployment
create_backup() {
    if [ -d "./data" ] || docker ps -q --filter "name=uaida" | grep -q .; then
        log_info "Creating backup..."
        mkdir -p "$BACKUP_DIR"
        
        # Backup data volumes
        if [ -d "./data" ]; then
            cp -r ./data "$BACKUP_DIR/"
        fi
        
        # Export current containers
        docker-compose ps -q | xargs -I {} docker export {} > "$BACKUP_DIR/containers_backup.tar" 2>/dev/null || true
        
        log_success "Backup created at $BACKUP_DIR"
    fi
}

# Build and deploy services
deploy_services() {
    log_info "Building and deploying services..."
    
    # Pull latest images
    docker-compose pull
    
    # Build custom images
    docker-compose build --no-cache
    
    # Start services
    docker-compose up -d
    
    log_success "Services deployed"
}

# Wait for services to be healthy
wait_for_health() {
    log_info "Waiting for services to be healthy..."
    
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if curl -f http://localhost:8080/health >/dev/null 2>&1; then
            log_success "UAIDA backend is healthy"
            break
        fi
        
        log_info "Attempt $attempt/$max_attempts - waiting for backend..."
        sleep 10
        ((attempt++))
    done
    
    if [ $attempt -gt $max_attempts ]; then
        log_error "Backend health check failed"
        exit 1
    fi
    
    # Check Grafana
    attempt=1
    while [ $attempt -le $max_attempts ]; do
        if curl -f http://localhost:3000/api/health >/dev/null 2>&1; then
            log_success "Grafana is healthy"
            break
        fi
        
        log_info "Attempt $attempt/$max_attempts - waiting for Grafana..."
        sleep 5
        ((attempt++))
    done
}

# Run post-deployment tests
run_tests() {
    log_info "Running post-deployment tests..."
    
    # Test API endpoints
    local endpoints=(
        "http://localhost:8080/health"
        "http://localhost:8080/metrics"
        "http://localhost:8080/docs"
    )
    
    for endpoint in "${endpoints[@]}"; do
        if curl -f "$endpoint" >/dev/null 2>&1; then
            log_success "✓ $endpoint"
        else
            log_error "✗ $endpoint"
        fi
    done
    
    # Test Postman collection if available
    if [ -f "postman_collection.json" ] && command -v newman &> /dev/null; then
        log_info "Running API tests with Newman..."
        newman run postman_collection.json --environment postman_environment.json || log_warning "Some API tests failed"
    fi
}

# Setup monitoring alerts
setup_monitoring() {
    log_info "Setting up monitoring and alerts..."
    
    # Create Grafana datasource
    curl -X POST \
        -H "Content-Type: application/json" \
        -d '{
            "name": "Prometheus",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "access": "proxy",
            "isDefault": true
        }' \
        http://admin:uaida_admin@localhost:3000/api/datasources 2>/dev/null || log_warning "Grafana datasource setup failed"
    
    log_success "Monitoring setup completed"
}

# Display deployment summary
show_summary() {
    log_success "🎉 UAIDA Deployment Completed Successfully!"
    echo ""
    echo "📊 Service URLs:"
    echo "  • UAIDA API:      http://localhost:8080"
    echo "  • API Docs:       http://localhost:8080/docs"
    echo "  • Metrics:        http://localhost:8080/metrics"
    echo "  • Grafana:        http://localhost:3000 (admin/uaida_admin)"
    echo "  • Prometheus:     http://localhost:9090"
    echo ""
    echo "🔧 Management Commands:"
    echo "  • View logs:      docker-compose logs -f"
    echo "  • Stop services:  docker-compose down"
    echo "  • Update:         ./scripts/deploy.sh"
    echo "  • Backup:         ./scripts/backup.sh"
    echo ""
    echo "📈 Monitoring:"
    echo "  • Grafana dashboard configured with UAIDA metrics"
    echo "  • Prometheus collecting metrics every 15s"
    echo "  • Health checks enabled for all services"
    echo ""
    
    if [ -n "$BACKUP_DIR" ]; then
        echo "💾 Backup Location: $BACKUP_DIR"
        echo ""
    fi
}

# Rollback function
rollback() {
    log_warning "Rolling back deployment..."
    
    if [ -z "$1" ]; then
        log_error "Please specify backup directory: ./scripts/deploy.sh rollback <backup_dir>"
        exit 1
    fi
    
    local backup_dir="$1"
    
    if [ ! -d "$backup_dir" ]; then
        log_error "Backup directory not found: $backup_dir"
        exit 1
    fi
    
    # Stop current services
    docker-compose down
    
    # Restore data
    if [ -d "$backup_dir/data" ]; then
        rm -rf ./data
        cp -r "$backup_dir/data" ./
    fi
    
    # Restart services
    docker-compose up -d
    
    log_success "Rollback completed"
}

# Main deployment flow
main() {
    case "${1:-deploy}" in
        "rollback")
            rollback "$2"
            ;;
        "deploy"|"")
            check_prerequisites
            create_backup
            deploy_services
            wait_for_health
            run_tests
            setup_monitoring
            show_summary
            ;;
        "test")
            run_tests
            ;;
        "health")
            wait_for_health
            ;;
        *)
            echo "Usage: $0 [deploy|rollback|test|health]"
            echo ""
            echo "Commands:"
            echo "  deploy    - Full deployment (default)"
            echo "  rollback  - Rollback to previous version"
            echo "  test      - Run post-deployment tests"
            echo "  health    - Check service health"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
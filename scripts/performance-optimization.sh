#!/bin/bash

# UAIDA Performance Optimization Script
set -e

echo "⚡ Starting UAIDA Performance Optimization..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Performance benchmarking
run_benchmarks() {
    log_info "Running performance benchmarks..."
    
    # Create benchmark results directory
    mkdir -p ./benchmarks/$(date +%Y%m%d_%H%M%S)
    local benchmark_dir="./benchmarks/$(date +%Y%m%d_%H%M%S)"
    
    # API endpoint benchmarks
    log_info "Benchmarking API endpoints..."
    
    # Health check benchmark
    ab -n 1000 -c 10 http://localhost:8080/health > "$benchmark_dir/health_benchmark.txt" 2>&1 || log_warning "Health benchmark failed"
    
    # Metrics endpoint benchmark
    ab -n 100 -c 5 http://localhost:8080/metrics > "$benchmark_dir/metrics_benchmark.txt" 2>&1 || log_warning "Metrics benchmark failed"
    
    # Agent Loop benchmark (if available)
    if command -v newman &> /dev/null && [ -f "postman_collection.json" ]; then
        log_info "Running Agent Loop performance test..."
        newman run postman_collection.json \
            --iteration-count 10 \
            --reporters cli,json \
            --reporter-json-export "$benchmark_dir/agent_loop_benchmark.json" || log_warning "Agent Loop benchmark failed"
    fi
    
    log_success "Benchmarks completed - Results in $benchmark_dir"
}

# Memory optimization
optimize_memory() {
    log_info "Applying memory optimizations..."
    
    # Update Docker Compose with optimized memory settings
    cat > docker-compose.override.yml << EOF
version: '3.8'
services:
  uaida-backend:
    deploy:
      resources:
        limits:
          memory: 2G
        reservations:
          memory: 1G
    environment:
      - RUST_LOG=info
      - UAIDA_MEMORY_POOL_SIZE=512MB
      - UAIDA_CACHE_SIZE=256MB
      - UAIDA_MAX_CONCURRENT_REQUESTS=50
    
  postgres:
    deploy:
      resources:
        limits:
          memory: 1G
        reservations:
          memory: 512M
    environment:
      - POSTGRES_SHARED_BUFFERS=256MB
      - POSTGRES_EFFECTIVE_CACHE_SIZE=768MB
      - POSTGRES_WORK_MEM=16MB
    
  redis:
    deploy:
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M
    command: redis-server --maxmemory 256mb --maxmemory-policy allkeys-lru --appendonly yes
EOF
    
    log_success "Memory optimization settings applied"
}

# CPU optimization
optimize_cpu() {
    log_info "Applying CPU optimizations..."
    
    # Get CPU count
    local cpu_count=$(nproc)
    local worker_count=$((cpu_count * 2))
    
    # Update backend configuration for optimal CPU usage
    cat >> docker-compose.override.yml << EOF
  uaida-backend:
    environment:
      - UAIDA_WORKER_THREADS=${worker_count}
      - UAIDA_BLOCKING_THREADS=16
      - TOKIO_WORKER_THREADS=${worker_count}
    deploy:
      resources:
        limits:
          cpus: '${cpu_count}.0'
        reservations:
          cpus: '0.5'
EOF
    
    log_success "CPU optimization settings applied (${worker_count} workers)"
}

# Database optimization
optimize_database() {
    log_info "Applying database optimizations..."
    
    # Create optimized PostgreSQL configuration
    cat > ./infra/postgres/postgresql.conf << EOF
# UAIDA Optimized PostgreSQL Configuration

# Memory settings
shared_buffers = 256MB
effective_cache_size = 768MB
work_mem = 16MB
maintenance_work_mem = 64MB

# Checkpoint settings
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100

# Connection settings
max_connections = 100
shared_preload_libraries = 'pg_stat_statements'

# Performance settings
random_page_cost = 1.1
effective_io_concurrency = 200
max_worker_processes = 8
max_parallel_workers_per_gather = 4
max_parallel_workers = 8
max_parallel_maintenance_workers = 4

# Logging
log_min_duration_statement = 1000
log_checkpoints = on
log_connections = on
log_disconnections = on
log_lock_waits = on
EOF
    
    # Update docker-compose to use custom config
    cat >> docker-compose.override.yml << EOF
  postgres:
    volumes:
      - ./infra/postgres/postgresql.conf:/etc/postgresql/postgresql.conf
    command: postgres -c config_file=/etc/postgresql/postgresql.conf
EOF
    
    log_success "Database optimization settings applied"
}

# Caching optimization
optimize_caching() {
    log_info "Setting up advanced caching..."
    
    # Redis optimization
    cat > ./infra/redis/redis.conf << EOF
# UAIDA Optimized Redis Configuration

# Memory management
maxmemory 256mb
maxmemory-policy allkeys-lru
maxmemory-samples 5

# Persistence
save 900 1
save 300 10
save 60 10000
appendonly yes
appendfsync everysec

# Performance
tcp-keepalive 300
timeout 0
tcp-backlog 511
databases 16

# Optimization
hash-max-ziplist-entries 512
hash-max-ziplist-value 64
list-max-ziplist-size -2
list-compress-depth 0
set-max-intset-entries 512
zset-max-ziplist-entries 128
zset-max-ziplist-value 64
EOF
    
    # Update docker-compose for Redis config
    cat >> docker-compose.override.yml << EOF
  redis:
    volumes:
      - ./infra/redis/redis.conf:/usr/local/etc/redis/redis.conf
    command: redis-server /usr/local/etc/redis/redis.conf
EOF
    
    log_success "Caching optimization settings applied"
}

# Network optimization
optimize_network() {
    log_info "Applying network optimizations..."
    
    # Update Nginx for better performance
    cat > ./infra/nginx/nginx-optimized.conf << EOF
worker_processes auto;
worker_rlimit_nofile 65535;

events {
    worker_connections 4096;
    use epoll;
    multi_accept on;
}

http {
    # Basic settings
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    keepalive_requests 1000;
    types_hash_max_size 2048;
    
    # Compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types
        text/plain
        text/css
        text/xml
        text/javascript
        application/json
        application/javascript
        application/xml+rss
        application/atom+xml;
    
    # Caching
    open_file_cache max=10000 inactive=20s;
    open_file_cache_valid 30s;
    open_file_cache_min_uses 2;
    open_file_cache_errors on;
    
    # Rate limiting (optimized)
    limit_req_zone \$binary_remote_addr zone=api:10m rate=20r/s;
    limit_req_zone \$binary_remote_addr zone=auth:10m rate=10r/s;
    
    # Connection pooling
    upstream uaida_backend {
        server uaida-backend:8080 max_fails=3 fail_timeout=30s;
        keepalive 32;
    }
    
    server {
        listen 80;
        server_name localhost;
        
        # API endpoints with optimized settings
        location /api/ {
            limit_req zone=api burst=50 nodelay;
            
            proxy_pass http://uaida_backend;
            proxy_http_version 1.1;
            proxy_set_header Connection "";
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
            
            # Optimized timeouts
            proxy_connect_timeout 5s;
            proxy_send_timeout 30s;
            proxy_read_timeout 30s;
            
            # Buffering
            proxy_buffering on;
            proxy_buffer_size 4k;
            proxy_buffers 8 4k;
            proxy_busy_buffers_size 8k;
        }
        
        # Static content caching
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg)$ {
            expires 1y;
            add_header Cache-Control "public, immutable";
            add_header Vary Accept-Encoding;
        }
    }
}
EOF
    
    log_success "Network optimization settings applied"
}

# Apply all optimizations
apply_optimizations() {
    log_info "Applying all performance optimizations..."
    
    optimize_memory
    optimize_cpu
    optimize_database
    optimize_caching
    optimize_network
    
    # Restart services with new configuration
    log_info "Restarting services with optimized configuration..."
    docker-compose down
    docker-compose up -d
    
    # Wait for services to be ready
    sleep 30
    
    log_success "All optimizations applied and services restarted"
}

# Performance monitoring setup
setup_performance_monitoring() {
    log_info "Setting up performance monitoring..."
    
    # Create performance monitoring dashboard
    cat > ./infra/grafana/dashboards/performance-dashboard.json << EOF
{
  "dashboard": {
    "title": "UAIDA Performance Monitoring",
    "panels": [
      {
        "title": "Response Time Percentiles",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, sum(rate(http_request_duration_ms_bucket[5m])) by (le))",
            "legendFormat": "99th percentile"
          },
          {
            "expr": "histogram_quantile(0.95, sum(rate(http_request_duration_ms_bucket[5m])) by (le))",
            "legendFormat": "95th percentile"
          },
          {
            "expr": "histogram_quantile(0.50, sum(rate(http_request_duration_ms_bucket[5m])) by (le))",
            "legendFormat": "50th percentile"
          }
        ]
      },
      {
        "title": "Throughput (RPS)",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total[1m]))",
            "legendFormat": "Requests per second"
          }
        ]
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))",
            "legendFormat": "Error rate"
          }
        ]
      },
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "process_resident_memory_bytes",
            "legendFormat": "Memory usage"
          }
        ]
      }
    ]
  }
}
EOF
    
    log_success "Performance monitoring dashboard created"
}

# Generate performance report
generate_performance_report() {
    log_info "Generating performance report..."
    
    local report_file="./reports/performance_report_$(date +%Y%m%d_%H%M%S).md"
    mkdir -p ./reports
    
    cat > "$report_file" << EOF
# UAIDA Performance Report
Generated: $(date)

## System Configuration
- CPU Cores: $(nproc)
- Memory: $(free -h | awk '/^Mem:/ {print $2}')
- Docker Version: $(docker --version)

## Performance Metrics

### API Response Times
$(curl -s http://localhost:8080/metrics | grep http_request_duration | head -10)

### Throughput
$(curl -s http://localhost:8080/metrics | grep http_requests_total | head -5)

### Resource Usage
$(docker stats --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}")

## Optimization Status
✅ Memory optimization applied
✅ CPU optimization applied  
✅ Database optimization applied
✅ Caching optimization applied
✅ Network optimization applied

## Recommendations
1. Monitor response times during peak usage
2. Scale horizontally if CPU usage > 80%
3. Increase memory if cache hit rate < 90%
4. Review slow queries in PostgreSQL logs

EOF
    
    log_success "Performance report generated: $report_file"
}

# Main function
main() {
    case "${1:-all}" in
        "benchmark")
            run_benchmarks
            ;;
        "memory")
            optimize_memory
            ;;
        "cpu")
            optimize_cpu
            ;;
        "database")
            optimize_database
            ;;
        "cache")
            optimize_caching
            ;;
        "network")
            optimize_network
            ;;
        "monitor")
            setup_performance_monitoring
            ;;
        "report")
            generate_performance_report
            ;;
        "all")
            run_benchmarks
            apply_optimizations
            setup_performance_monitoring
            generate_performance_report
            log_success "🚀 Performance optimization completed!"
            ;;
        *)
            echo "Usage: $0 [benchmark|memory|cpu|database|cache|network|monitor|report|all]"
            exit 1
            ;;
    esac
}

main "$@"
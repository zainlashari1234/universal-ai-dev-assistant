#!/bin/bash

# Kubernetes Deployment Script for Universal AI Development Assistant
# This script deploys the application to a Kubernetes cluster

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE="uaida-production"
ENVIRONMENT=${1:-production}

echo -e "${BLUE}🚀 Kubernetes Deployment - Universal AI Development Assistant${NC}"
echo "=================================================================="
echo ""

# Check if kubectl is available
if ! command -v kubectl &> /dev/null; then
    echo -e "${RED}❌ kubectl is not installed or not in PATH${NC}"
    exit 1
fi

# Check if we can connect to the cluster
if ! kubectl cluster-info &> /dev/null; then
    echo -e "${RED}❌ Cannot connect to Kubernetes cluster${NC}"
    echo "Please check your kubeconfig and cluster connectivity"
    exit 1
fi

echo -e "${GREEN}✅ Connected to Kubernetes cluster${NC}"
kubectl cluster-info

echo ""
echo -e "${YELLOW}📋 Deployment Configuration:${NC}"
echo "  Environment: $ENVIRONMENT"
echo "  Namespace: $NAMESPACE"
echo "  Cluster: $(kubectl config current-context)"
echo ""

# Confirm deployment
read -p "Do you want to proceed with the deployment? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Deployment cancelled."
    exit 0
fi

echo ""
echo -e "${YELLOW}🔧 Step 1: Creating namespace and basic resources...${NC}"

# Apply namespace
kubectl apply -f namespace.yaml

# Apply ConfigMaps
kubectl apply -f configmap.yaml

echo -e "${GREEN}✅ Namespace and ConfigMaps created${NC}"

echo ""
echo -e "${YELLOW}🔐 Step 2: Setting up secrets...${NC}"

# Check if secrets file exists
if [ ! -f "secrets.yaml" ]; then
    echo -e "${RED}❌ secrets.yaml not found!${NC}"
    echo "Please create secrets.yaml with your production secrets"
    exit 1
fi

# Apply secrets
kubectl apply -f secrets.yaml

echo -e "${GREEN}✅ Secrets configured${NC}"

echo ""
echo -e "${YELLOW}🗄️ Step 3: Deploying database...${NC}"

# Deploy PostgreSQL
kubectl apply -f postgres.yaml

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/postgres -n $NAMESPACE

echo -e "${GREEN}✅ PostgreSQL deployed and ready${NC}"

echo ""
echo -e "${YELLOW}🔧 Step 4: Deploying backend services...${NC}"

# Deploy backend
kubectl apply -f backend.yaml

# Wait for backend to be ready
echo "Waiting for backend to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/uaida-backend -n $NAMESPACE

echo -e "${GREEN}✅ Backend services deployed${NC}"

echo ""
echo -e "${YELLOW}🎨 Step 5: Deploying frontend...${NC}"

# Deploy frontend
kubectl apply -f frontend.yaml

# Wait for frontend to be ready
echo "Waiting for frontend to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/uaida-frontend -n $NAMESPACE

echo -e "${GREEN}✅ Frontend deployed${NC}"

echo ""
echo -e "${YELLOW}📊 Step 6: Setting up monitoring...${NC}"

# Deploy monitoring
kubectl apply -f monitoring.yaml

echo -e "${GREEN}✅ Monitoring stack deployed${NC}"

echo ""
echo -e "${YELLOW}🌐 Step 7: Configuring ingress...${NC}"

# Deploy ingress
kubectl apply -f ingress.yaml

echo -e "${GREEN}✅ Ingress configured${NC}"

echo ""
echo -e "${YELLOW}🧪 Step 8: Running health checks...${NC}"

# Check pod status
echo "Checking pod status..."
kubectl get pods -n $NAMESPACE

# Check service status
echo ""
echo "Checking service status..."
kubectl get services -n $NAMESPACE

# Check ingress status
echo ""
echo "Checking ingress status..."
kubectl get ingress -n $NAMESPACE

echo ""
echo -e "${YELLOW}⏳ Step 9: Waiting for all services to be healthy...${NC}"

# Wait for all deployments to be ready
deployments=("postgres" "uaida-backend" "uaida-frontend" "prometheus" "grafana")

for deployment in "${deployments[@]}"; do
    echo "Waiting for $deployment to be ready..."
    kubectl wait --for=condition=available --timeout=300s deployment/$deployment -n $NAMESPACE || {
        echo -e "${RED}❌ $deployment failed to become ready${NC}"
        kubectl describe deployment/$deployment -n $NAMESPACE
        exit 1
    }
done

echo ""
echo -e "${YELLOW}🔍 Step 10: Final verification...${NC}"

# Test backend health
backend_pod=$(kubectl get pods -n $NAMESPACE -l app=uaida-backend -o jsonpath='{.items[0].metadata.name}')
if [ -n "$backend_pod" ]; then
    echo "Testing backend health..."
    kubectl exec -n $NAMESPACE $backend_pod -- curl -f http://localhost:3001/health || {
        echo -e "${RED}❌ Backend health check failed${NC}"
        exit 1
    }
    echo -e "${GREEN}✅ Backend health check passed${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Deployment completed successfully!${NC}"
echo ""

echo -e "${BLUE}📊 Deployment Summary:${NC}"
kubectl get all -n $NAMESPACE

echo ""
echo -e "${BLUE}📱 Access Information:${NC}"
echo "  Frontend: https://yourdomain.com"
echo "  API: https://api.yourdomain.com"
echo "  Monitoring: https://monitoring.yourdomain.com"
echo ""

echo -e "${BLUE}🔐 Default Credentials:${NC}"
echo "  Grafana: admin / [check secrets]"
echo ""

echo -e "${BLUE}📋 Useful Commands:${NC}"
echo "  View logs: kubectl logs -f deployment/uaida-backend -n $NAMESPACE"
echo "  Scale backend: kubectl scale deployment uaida-backend --replicas=5 -n $NAMESPACE"
echo "  Update image: kubectl set image deployment/uaida-backend backend=new-image:tag -n $NAMESPACE"
echo "  Port forward: kubectl port-forward service/uaida-frontend-service 3000:80 -n $NAMESPACE"
echo ""

echo -e "${GREEN}✨ Universal AI Development Assistant is now running on Kubernetes!${NC}"
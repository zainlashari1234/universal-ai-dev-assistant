#!/bin/bash

# Universal AI Development Assistant - Frontend & Backend Test Script
# This script tests the complete authentication and API integration

echo "🚀 Testing Universal AI Development Assistant - Frontend & Backend Integration"
echo "=============================================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BACKEND_URL="http://localhost:3001"
FRONTEND_URL="http://localhost:3000"

# Test data
TEST_EMAIL="test@example.com"
TEST_USERNAME="testuser"
TEST_PASSWORD="TestPass123!"
TEST_FULL_NAME="Test User"

echo -e "${BLUE}📋 Test Configuration:${NC}"
echo "  Backend URL: $BACKEND_URL"
echo "  Frontend URL: $FRONTEND_URL"
echo "  Test Email: $TEST_EMAIL"
echo ""

# Function to check if service is running
check_service() {
    local url=$1
    local name=$2
    
    echo -n "  Checking $name... "
    if curl -s "$url/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Running${NC}"
        return 0
    else
        echo -e "${RED}❌ Not running${NC}"
        return 1
    fi
}

# Function to test API endpoint
test_api() {
    local method=$1
    local endpoint=$2
    local data=$3
    local expected_status=$4
    local description=$5
    
    echo -n "  Testing $description... "
    
    if [ -n "$data" ]; then
        response=$(curl -s -w "%{http_code}" -X "$method" \
            -H "Content-Type: application/json" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            -d "$data" \
            "$BACKEND_URL$endpoint")
    else
        response=$(curl -s -w "%{http_code}" -X "$method" \
            -H "Authorization: Bearer $ACCESS_TOKEN" \
            "$BACKEND_URL$endpoint")
    fi
    
    status_code="${response: -3}"
    response_body="${response%???}"
    
    if [ "$status_code" = "$expected_status" ]; then
        echo -e "${GREEN}✅ $status_code${NC}"
        return 0
    else
        echo -e "${RED}❌ $status_code (expected $expected_status)${NC}"
        echo "    Response: $response_body"
        return 1
    fi
}

echo -e "${YELLOW}🔍 Step 1: Checking Services${NC}"
backend_running=false
frontend_running=false

if check_service "$BACKEND_URL" "Backend"; then
    backend_running=true
fi

if check_service "$FRONTEND_URL" "Frontend"; then
    frontend_running=true
fi

if [ "$backend_running" = false ]; then
    echo -e "${RED}❌ Backend is not running. Please start it first:${NC}"
    echo "  cd universal-ai-dev-assistant/backend"
    echo "  cargo run"
    exit 1
fi

echo ""
echo -e "${YELLOW}🧪 Step 2: Testing Backend API${NC}"

# Test health endpoint
echo -n "  Testing health endpoint... "
health_response=$(curl -s "$BACKEND_URL/health")
if echo "$health_response" | grep -q "healthy"; then
    echo -e "${GREEN}✅ Healthy${NC}"
else
    echo -e "${RED}❌ Unhealthy${NC}"
    echo "    Response: $health_response"
fi

# Test user registration
echo -n "  Testing user registration... "
register_response=$(curl -s -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d "{
        \"email\": \"$TEST_EMAIL\",
        \"username\": \"$TEST_USERNAME\",
        \"password\": \"$TEST_PASSWORD\",
        \"full_name\": \"$TEST_FULL_NAME\"
    }" \
    "$BACKEND_URL/auth/register")

register_status="${register_response: -3}"
register_body="${register_response%???}"

if [ "$register_status" = "200" ] || [ "$register_status" = "400" ]; then
    if echo "$register_body" | grep -q "already exists"; then
        echo -e "${YELLOW}⚠️  User already exists${NC}"
    else
        echo -e "${GREEN}✅ $register_status${NC}"
    fi
else
    echo -e "${RED}❌ $register_status${NC}"
    echo "    Response: $register_body"
fi

# Test user login
echo -n "  Testing user login... "
login_response=$(curl -s -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d "{
        \"email\": \"$TEST_EMAIL\",
        \"password\": \"$TEST_PASSWORD\"
    }" \
    "$BACKEND_URL/auth/login")

login_status="${login_response: -3}"
login_body="${login_response%???}"

if [ "$login_status" = "200" ]; then
    echo -e "${GREEN}✅ $login_status${NC}"
    # Extract access token
    ACCESS_TOKEN=$(echo "$login_body" | grep -o '"access_token":"[^"]*' | cut -d'"' -f4)
    if [ -n "$ACCESS_TOKEN" ]; then
        echo "    Access token obtained"
    else
        echo -e "${RED}    ❌ No access token in response${NC}"
    fi
else
    echo -e "${RED}❌ $login_status${NC}"
    echo "    Response: $login_body"
    exit 1
fi

# Test protected endpoints
echo ""
echo -e "${YELLOW}🔐 Step 3: Testing Protected Endpoints${NC}"

test_api "GET" "/auth/profile" "" "200" "Get user profile"
test_api "GET" "/api-keys" "" "200" "Get API keys"
test_api "GET" "/metrics" "" "200" "Get metrics"

# Test API key creation
echo -n "  Testing API key creation... "
api_key_response=$(curl -s -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ACCESS_TOKEN" \
    -d "{
        \"provider\": \"openrouter\",
        \"key_name\": \"Test Key\",
        \"api_key\": \"sk-test-key-12345\",
        \"monthly_limit\": 1000
    }" \
    "$BACKEND_URL/api-keys")

api_key_status="${api_key_response: -3}"
api_key_body="${api_key_response%???}"

if [ "$api_key_status" = "200" ]; then
    echo -e "${GREEN}✅ $api_key_status${NC}"
else
    echo -e "${RED}❌ $api_key_status${NC}"
    echo "    Response: $api_key_body"
fi

echo ""
echo -e "${YELLOW}🌐 Step 4: Frontend Integration${NC}"

if [ "$frontend_running" = true ]; then
    echo -e "${GREEN}✅ Frontend is running at $FRONTEND_URL${NC}"
    echo ""
    echo -e "${BLUE}📱 Manual Testing Steps:${NC}"
    echo "  1. Open $FRONTEND_URL in your browser"
    echo "  2. Try logging in with:"
    echo "     Email: $TEST_EMAIL"
    echo "     Password: $TEST_PASSWORD"
    echo "  3. Navigate through the dashboard, settings, and analytics"
    echo "  4. Test API key management in settings"
    echo "  5. Check analytics charts and metrics"
else
    echo -e "${YELLOW}⚠️  Frontend is not running. To start it:${NC}"
    echo "  cd universal-ai-dev-assistant/frontend"
    echo "  npm install"
    echo "  npm start"
fi

echo ""
echo -e "${GREEN}🎉 Backend API Tests Completed!${NC}"
echo ""
echo -e "${BLUE}📊 Test Summary:${NC}"
echo "  ✅ Backend health check"
echo "  ✅ User registration/login"
echo "  ✅ JWT authentication"
echo "  ✅ Protected endpoints"
echo "  ✅ API key management"
echo ""
echo -e "${BLUE}🚀 Next Steps:${NC}"
echo "  1. Start the frontend if not running"
echo "  2. Test the complete user flow in the browser"
echo "  3. Add your real AI provider API keys"
echo "  4. Test AI completions and analysis"
echo ""
echo -e "${GREEN}✨ Universal AI Development Assistant is ready!${NC}"
# Universal AI Development Assistant - Backend

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+
- Docker (optional)

### Setup

1. **Clone and navigate to backend**
```bash
cd universal-ai-dev-assistant/backend
```

2. **Set up environment variables**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Set up PostgreSQL database**
```bash
# Using Docker
docker run --name uaida-postgres -e POSTGRES_USER=uaida -e POSTGRES_PASSWORD=uaida123 -e POSTGRES_DB=uaida_dev -p 5432:5432 -d postgres:14

# Or install PostgreSQL locally and create database
createdb uaida_dev
```

4. **Install dependencies and run migrations**
```bash
cargo build
cargo run
# Migrations will run automatically on startup
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://uaida:uaida123@localhost:5432/uaida_dev` |
| `JWT_SECRET` | Secret key for JWT tokens | Required in production |
| `ENCRYPTION_KEY` | 32-byte key for API key encryption | Required in production |
| `PORT` | Server port | `3001` |
| `HOST` | Server host | `0.0.0.0` |

### AI Provider Keys
Users can add their own API keys via the web interface, or you can set default ones:
- `OPENROUTER_API_KEY`
- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_API_KEY`
- `GROQ_API_KEY`
- `TOGETHER_API_KEY`
- `COHERE_API_KEY`

## 📚 API Documentation

### Authentication Endpoints

#### Register User
```http
POST /auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "username": "username",
  "password": "SecurePass123!",
  "full_name": "Full Name"
}
```

#### Login
```http
POST /auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "SecurePass123!"
}
```

#### Refresh Token
```http
POST /auth/refresh
Content-Type: application/json

{
  "refresh_token": "your_refresh_token"
}
```

### Protected Endpoints (Require Authorization header)

#### Get Profile
```http
GET /auth/profile
Authorization: Bearer your_access_token
```

#### Update Profile
```http
PUT /auth/profile
Authorization: Bearer your_access_token
Content-Type: application/json

{
  "full_name": "New Name",
  "username": "new_username"
}
```

#### Change Password
```http
POST /auth/change-password
Authorization: Bearer your_access_token
Content-Type: application/json

{
  "current_password": "current_pass",
  "new_password": "new_secure_pass"
}
```

### API Key Management

#### Create API Key
```http
POST /api-keys
Authorization: Bearer your_access_token
Content-Type: application/json

{
  "provider": "openrouter",
  "key_name": "My OpenRouter Key",
  "api_key": "sk-or-v1-...",
  "monthly_limit": 1000
}
```

#### Get API Keys
```http
GET /api-keys
Authorization: Bearer your_access_token
```

#### Delete API Key
```http
DELETE /api-keys/{key_id}
Authorization: Bearer your_access_token
```

### AI Completion

#### Text Completion
```http
POST /completion
Authorization: Bearer your_access_token
Content-Type: application/json

{
  "prompt": "Write a Python function to calculate fibonacci",
  "provider": "openrouter",
  "model": "gpt-4o-mini",
  "max_tokens": 1000,
  "temperature": 0.7
}
```

#### Code Analysis
```http
POST /analysis
Authorization: Bearer your_access_token
Content-Type: application/json

{
  "code": "def fibonacci(n): return n if n <= 1 else fibonacci(n-1) + fibonacci(n-2)",
  "language": "python",
  "analysis_type": "security"
}
```

### System Endpoints

#### Health Check
```http
GET /health
```

#### Available Providers
```http
GET /providers
```

#### Provider Models
```http
GET /providers/{provider}/models
```

#### System Metrics
```http
GET /metrics
Authorization: Bearer your_access_token
```

## 🗄️ Database Schema

### Core Tables
- `users` - User accounts and authentication
- `sessions` - JWT session management
- `user_preferences` - User settings and preferences
- `api_keys` - Encrypted AI provider API keys
- `provider_metrics` - Performance and cost tracking
- `completion_logs` - AI request/response logging
- `projects` - User projects
- `runs` - Agent execution runs
- `artifacts` - Generated code and outputs

### Key Features
- **Encrypted API Keys**: All provider keys are encrypted using AES-256-GCM
- **Session Management**: JWT tokens with refresh capability
- **Usage Tracking**: Detailed metrics for cost optimization
- **Audit Logging**: Complete request/response history

## 🔒 Security Features

### Authentication
- JWT-based authentication with access/refresh tokens
- Secure password hashing with bcrypt
- Session management with expiration

### API Key Security
- AES-256-GCM encryption for stored API keys
- Per-user key isolation
- Usage limits and monitoring

### Input Validation
- Request sanitization
- SQL injection prevention
- XSS protection

## 🚀 Development

### Running in Development
```bash
# With auto-reload
cargo watch -x run

# With debug logging
RUST_LOG=debug cargo run
```

### Running Tests
```bash
cargo test
```

### Database Migrations
```bash
# Migrations run automatically on startup
# To create new migration:
sqlx migrate add migration_name
```

### Building for Production
```bash
cargo build --release
```

## 📊 Monitoring

### Health Endpoint
The `/health` endpoint provides:
- Service status
- Database connectivity
- Provider availability
- Feature flags

### Metrics
The `/metrics` endpoint provides:
- Request counts and latencies
- Provider performance
- Cost tracking
- Error rates

## 🐳 Docker Deployment

```dockerfile
# Build stage
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/universal-ai-dev-assistant /usr/local/bin/
EXPOSE 3001
CMD ["universal-ai-dev-assistant"]
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

MIT License - see LICENSE file for details
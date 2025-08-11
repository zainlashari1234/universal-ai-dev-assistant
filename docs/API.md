# Universal AI Development Assistant - API Documentation

## Overview

The UAIDA API provides endpoints for AI-powered code completion, analysis, and development assistance.

**Base URL**: `http://localhost:8080`

## Authentication

Currently, the API does not require authentication for local development. Enterprise versions will include API key authentication.

## Endpoints

### Health Check

Check the server status and capabilities.

**GET** `/health`

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "ai_model_loaded": true,
  "supported_languages": [
    "python",
    "javascript",
    "typescript",
    "rust",
    "go",
    "java",
    "cpp",
    "c"
  ]
}
```

### Code Completion

Get AI-powered code completion suggestions.

**POST** `/api/v1/complete`

**Request Body:**
```json
{
  "code": "def fibonacci(n):\n    if n <= 1:\n        return n\n    return ",
  "language": "python",
  "cursor_position": 65,
  "context": "optional additional context"
}
```

**Response:**
```json
{
  "suggestions": [
    "fibonacci(n-1) + fibonacci(n-2)",
    "fib(n-1) + fib(n-2)",
    "fibonacci(n - 1) + fibonacci(n - 2)"
  ],
  "confidence": 0.85,
  "processing_time_ms": 45
}
```

### Code Analysis

Analyze code for issues, improvements, and security vulnerabilities.

**POST** `/api/v1/analyze`

**Request Body:**
```json
{
  "code": "password = 'admin123'\neval('print(\"hello\")')",
  "language": "python",
  "cursor_position": 0
}
```

**Response:**
```json
{
  "issues": [
    {
      "type": "security",
      "message": "Hardcoded password detected",
      "line": 1,
      "severity": "high"
    },
    {
      "type": "security", 
      "message": "Use of eval() can lead to code injection",
      "line": 2,
      "severity": "critical"
    }
  ],
  "suggestions": [
    {
      "type": "security",
      "message": "Use environment variables for sensitive data",
      "line": 1
    },
    {
      "type": "security",
      "message": "Use ast.literal_eval() for safe evaluation",
      "line": 2
    }
  ],
  "complexity": {
    "cyclomatic": 1,
    "cognitive": 1,
    "maintainability_index": 75
  },
  "security_concerns": [
    {
      "severity": "high",
      "category": "hardcoded_secrets",
      "count": 1
    },
    {
      "severity": "critical", 
      "category": "code_injection",
      "count": 1
    }
  ],
  "test_coverage": 0,
  "documentation_score": 20
}
```

## Error Responses

All endpoints return appropriate HTTP status codes:

- `200 OK` - Success
- `400 Bad Request` - Invalid request format
- `500 Internal Server Error` - Server error

**Error Response Format:**
```json
{
  "error": "Invalid request format",
  "code": "INVALID_REQUEST",
  "details": "Missing required field: language"
}
```

## Rate Limiting

- **Development**: No rate limiting
- **Production**: 1000 requests per hour per IP
- **Enterprise**: Custom limits based on subscription

## Supported Languages

| Language   | Code Completion | Analysis | Security Scan |
|------------|----------------|----------|---------------|
| Python     | ✅             | ✅       | ✅            |
| JavaScript | ✅             | ✅       | ✅            |
| TypeScript | ✅             | ✅       | ✅            |
| Rust       | ✅             | ✅       | ⚠️            |
| Go         | ✅             | ✅       | ⚠️            |
| Java       | ✅             | ⚠️       | ⚠️            |
| C++        | ⚠️             | ⚠️       | ❌            |
| C          | ⚠️             | ⚠️       | ❌            |

**Legend:**
- ✅ Full support
- ⚠️ Partial support
- ❌ Not yet supported

## WebSocket API (Coming Soon)

Real-time code analysis and completion via WebSocket connection.

**Connection:** `ws://localhost:8080/ws`

## SDK Libraries

Official SDKs are available for:

- **JavaScript/TypeScript**: `npm install @uaida/sdk`
- **Python**: `pip install uaida-sdk`
- **Rust**: `cargo add uaida-sdk`

## Examples

### Python SDK
```python
from uaida import UAIDAClient

client = UAIDAClient("http://localhost:8080")

# Get completion
result = await client.complete(
    code="def hello():",
    language="python",
    cursor_position=12
)

print(result.suggestions)
```

### JavaScript SDK
```javascript
import { UAIDAClient } from '@uaida/sdk';

const client = new UAIDAClient('http://localhost:8080');

// Analyze code
const analysis = await client.analyze({
    code: 'const password = "admin123";',
    language: 'javascript'
});

console.log(analysis.issues);
```

## Changelog

### v0.1.0
- Initial API release
- Basic code completion
- Security analysis
- Python and JavaScript support

### Roadmap

- **v0.2.0**: WebSocket support, more languages
- **v0.3.0**: Custom model support, team features
- **v1.0.0**: Enterprise features, advanced analytics
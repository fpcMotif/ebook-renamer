# Authentication API Reference

## Overview

The Authentication API provides secure user login, registration, and token management.

**Base URL:** `https://api.example.com/v1`

## Endpoints

### POST /auth/register

Create a new user account.

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecureP@ss123",
  "username": "johndoe"
}
```

**Response (201 Created):**
```json
{
  "user_id": "usr_1234567890",
  "email": "user@example.com",
  "username": "johndoe",
  "created_at": "2025-06-16T10:30:00Z"
}
```

### POST /auth/login

Authenticate user and receive access token.

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecureP@ss123"
}
```

**Response (200 OK):**
```json
{
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc...",
  "expires_in": 3600,
  "token_type": "Bearer"
}
```

### POST /auth/refresh

Refresh an expired access token.

**Headers:**
```
Authorization: Bearer {refresh_token}
```

**Response (200 OK):**
```json
{
  "access_token": "eyJhbGc...",
  "expires_in": 3600
}
```

## Error Codes

| Code | Description |
|------|-------------|
| 400 | Invalid request parameters |
| 401 | Invalid credentials |
| 403 | Account locked or disabled |
| 429 | Too many login attempts |
| 500 | Internal server error |

## Rate Limiting

- **Login attempts:** 5 per minute per IP
- **Registration:** 3 per hour per IP
- **Token refresh:** 10 per minute per user

## Security Headers

All requests must include:
```
X-API-Key: your_api_key
Content-Type: application/json
```

Authenticated requests also require:
```
Authorization: Bearer {access_token}
```

use super::*;
use anyhow::Result;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    issuer: String,
    access_token_expiry: Duration,
    refresh_token_expiry: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub email: String,      // User email
    pub org_id: String,     // Organization ID
    pub roles: Vec<String>, // User roles
    pub permissions: Vec<String>, // User permissions
    pub session_id: String, // Session ID
    pub iat: u64,          // Issued at
    pub exp: u64,          // Expiration
    pub iss: String,       // Issuer
    pub aud: String,       // Audience
    pub token_type: TokenType, // Token type
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

impl JwtManager {
    pub fn new(secret: &str, issuer: String) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_ref());
        let decoding_key = DecodingKey::from_secret(secret.as_ref());

        Self {
            encoding_key,
            decoding_key,
            algorithm: Algorithm::HS256,
            issuer,
            access_token_expiry: Duration::from_secs(3600), // 1 hour
            refresh_token_expiry: Duration::from_secs(86400 * 7), // 7 days
        }
    }

    pub fn generate_token_pair(&self, user: &User, session_id: Uuid) -> Result<TokenPair> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // Generate access token
        let access_claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            org_id: user.organization_id.to_string(),
            roles: user.roles.iter().map(|r| r.name.clone()).collect(),
            permissions: user.permissions.iter().map(|p| format!("{:?}", p)).collect(),
            session_id: session_id.to_string(),
            iat: now,
            exp: now + self.access_token_expiry.as_secs(),
            iss: self.issuer.clone(),
            aud: "uaida-api".to_string(),
            token_type: TokenType::Access,
        };

        // Generate refresh token
        let refresh_claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            org_id: user.organization_id.to_string(),
            roles: vec![], // Refresh tokens don't need roles
            permissions: vec![], // Refresh tokens don't need permissions
            session_id: session_id.to_string(),
            iat: now,
            exp: now + self.refresh_token_expiry.as_secs(),
            iss: self.issuer.clone(),
            aud: "uaida-refresh".to_string(),
            token_type: TokenType::Refresh,
        };

        let header = Header::new(self.algorithm);
        
        let access_token = encode(&header, &access_claims, &self.encoding_key)?;
        let refresh_token = encode(&header, &refresh_claims, &self.encoding_key)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            expires_in: self.access_token_expiry.as_secs(),
            token_type: "Bearer".to_string(),
        })
    }

    pub fn validate_token(&self, token: &str, expected_type: TokenType) -> Result<Claims> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&[&self.issuer]);
        
        match expected_type {
            TokenType::Access => validation.set_audience(&["uaida-api"]),
            TokenType::Refresh => validation.set_audience(&["uaida-refresh"]),
        }

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        
        // Verify token type matches expected
        match (&token_data.claims.token_type, &expected_type) {
            (TokenType::Access, TokenType::Access) | (TokenType::Refresh, TokenType::Refresh) => {},
            _ => return Err(anyhow::anyhow!("Token type mismatch")),
        }

        Ok(token_data.claims)
    }

    pub fn refresh_access_token(&self, refresh_token: &str, user: &User, session_id: Uuid) -> Result<TokenPair> {
        // Validate refresh token
        let refresh_claims = self.validate_token(refresh_token, TokenType::Refresh)?;
        
        // Verify the refresh token belongs to the user and session
        if refresh_claims.sub != user.id.to_string() || refresh_claims.session_id != session_id.to_string() {
            return Err(anyhow::anyhow!("Invalid refresh token"));
        }

        // Generate new token pair
        self.generate_token_pair(user, session_id)
    }

    pub fn extract_bearer_token(authorization_header: &str) -> Option<&str> {
        authorization_header
            .strip_prefix("Bearer ")
            .map(|token| token.trim())
    }

    pub fn get_token_expiry(&self, token_type: TokenType) -> Duration {
        match token_type {
            TokenType::Access => self.access_token_expiry,
            TokenType::Refresh => self.refresh_token_expiry,
        }
    }

    pub fn is_token_expired(&self, claims: &Claims) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        claims.exp <= now
    }

    pub fn get_token_remaining_time(&self, claims: &Claims) -> Duration {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if claims.exp > now {
            Duration::from_secs(claims.exp - now)
        } else {
            Duration::ZERO
        }
    }
}

impl Claims {
    pub fn to_auth_context(&self, ip_address: String, user_agent: String) -> Result<AuthContext> {
        let user_id = Uuid::parse_str(&self.sub)?;
        let org_id = Uuid::parse_str(&self.org_id)?;
        let session_id = Uuid::parse_str(&self.session_id)?;
        
        // Parse permissions back from strings
        let permissions: Vec<Permission> = self.permissions
            .iter()
            .filter_map(|p| match p.as_str() {
                "CreatePlan" => Some(Permission::CreatePlan),
                "ExecutePlan" => Some(Permission::ExecutePlan),
                "ViewPlan" => Some(Permission::ViewPlan),
                "CancelPlan" => Some(Permission::CancelPlan),
                "GenerateCode" => Some(Permission::GenerateCode),
                "ReviewCode" => Some(Permission::ReviewCode),
                "ApproveCode" => Some(Permission::ApproveCode),
                "DeployCode" => Some(Permission::DeployCode),
                "ViewSecurityReports" => Some(Permission::ViewSecurityReports),
                "OverrideSecurityBlocks" => Some(Permission::OverrideSecurityBlocks),
                "ConfigureSecurity" => Some(Permission::ConfigureSecurity),
                "ManageUsers" => Some(Permission::ManageUsers),
                "ManageRoles" => Some(Permission::ManageRoles),
                "ManageOrganization" => Some(Permission::ManageOrganization),
                "ViewAuditLogs" => Some(Permission::ViewAuditLogs),
                "ConfigureSystem" => Some(Permission::ConfigureSystem),
                "ApiAccess" => Some(Permission::ApiAccess),
                "ApiAdmin" => Some(Permission::ApiAdmin),
                "RunEvaluations" => Some(Permission::RunEvaluations),
                "ViewEvaluations" => Some(Permission::ViewEvaluations),
                "ConfigureEvaluations" => Some(Permission::ConfigureEvaluations),
                _ => {
                    warn!("Unknown permission in token: {}", p);
                    None
                }
            })
            .collect();

        // Create a minimal user object from claims
        let user = User {
            id: user_id,
            email: self.email.clone(),
            name: self.email.split('@').next().unwrap_or("Unknown").to_string(),
            organization_id: org_id,
            roles: vec![], // Roles are not stored in token for security
            permissions: permissions.clone(),
            created_at: chrono::Utc::now(), // Placeholder
            last_login: Some(chrono::Utc::now()),
            is_active: true,
        };

        Ok(AuthContext {
            user,
            session_id,
            ip_address,
            user_agent,
            expires_at: chrono::DateTime::from_timestamp(self.exp as i64, 0)
                .unwrap_or_else(chrono::Utc::now),
            permissions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let jwt_manager = JwtManager::new("test_secret", "test_issuer".to_string());
        let org_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            organization_id: org_id,
            roles: vec![Role::developer_role(org_id)],
            permissions: vec![Permission::ApiAccess, Permission::CreatePlan],
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // Generate token pair
        let token_pair = jwt_manager.generate_token_pair(&user, session_id).unwrap();
        
        // Validate access token
        let access_claims = jwt_manager
            .validate_token(&token_pair.access_token, TokenType::Access)
            .unwrap();
        
        assert_eq!(access_claims.sub, user.id.to_string());
        assert_eq!(access_claims.email, user.email);
        
        // Validate refresh token
        let refresh_claims = jwt_manager
            .validate_token(&token_pair.refresh_token, TokenType::Refresh)
            .unwrap();
        
        assert_eq!(refresh_claims.sub, user.id.to_string());
        assert_eq!(refresh_claims.email, user.email);
    }

    #[test]
    fn test_token_refresh() {
        let jwt_manager = JwtManager::new("test_secret", "test_issuer".to_string());
        let org_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        
        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            organization_id: org_id,
            roles: vec![Role::developer_role(org_id)],
            permissions: vec![Permission::ApiAccess],
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
        };

        // Generate initial token pair
        let initial_tokens = jwt_manager.generate_token_pair(&user, session_id).unwrap();
        
        // Refresh tokens
        let new_tokens = jwt_manager
            .refresh_access_token(&initial_tokens.refresh_token, &user, session_id)
            .unwrap();
        
        // Validate new access token
        let new_claims = jwt_manager
            .validate_token(&new_tokens.access_token, TokenType::Access)
            .unwrap();
        
        assert_eq!(new_claims.sub, user.id.to_string());
    }
}